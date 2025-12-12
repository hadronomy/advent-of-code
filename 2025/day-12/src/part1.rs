use bitvec::prelude::*;
use chumsky::prelude::*;
use miette::*;
use rayon::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Point {
    r: i8,
    c: i8,
}

#[derive(Debug, Clone)]
struct Shape {
    id: usize,
    area: usize,
    variants: Vec<Vec<Point>>,
}

#[derive(Debug, Clone)]
struct Region {
    width: usize,
    height: usize,
    reqs: Vec<usize>,
}

#[derive(Clone)]
enum LineSuffix {
    Shape(Vec<Point>),
    Region(usize, Vec<usize>),
}

#[derive(Debug)]
enum InputItem {
    Shape(Shape),
    Region(Region),
}

struct Solver {
    /// Precomputed valid placement masks for each shape ID.
    /// masks[shape_id] = Vec<(anchor_index, BitVec)>
    placements: Vec<Vec<(usize, BitVec)>>,
    /// Tasks to solve: (shape_id, count_needed)
    tasks: Vec<(usize, usize)>,
    /// Total number of cells in the grid
    total_cells: usize,
}

impl Solver {
    fn new(shapes: &[Shape], region: &Region) -> Option<Self> {
        let w = region.width;
        let h = region.height;
        let total_cells = w * h;

        let mut tasks = Vec::new();
        let mut total_area = 0;

        for (id, &count) in region.reqs.iter().enumerate() {
            if count == 0 {
                continue;
            }
            if id >= shapes.len() {
                return None;
            }

            let shape = &shapes[id];
            total_area += shape.area * count;
            tasks.push((id, count));
        }

        if total_area > total_cells {
            return None;
        }

        // Sort tasks by shape area (Largest First)
        tasks.sort_by_key(|&(id, _)| std::cmp::Reverse(shapes[id].area));

        // Precompute placement masks
        let mut placements = vec![Vec::new(); shapes.len()];

        for &(id, _) in &tasks {
            let shape = &shapes[id];
            let mut shape_masks = Vec::new();

            for variant in &shape.variants {
                for r in 0..(h as i8) {
                    for c in 0..(w as i8) {
                        // Check bounds and build mask
                        let mut valid = true;
                        let mut mask = BitVec::<usize, Lsb0>::repeat(false, total_cells);

                        for p in variant {
                            let nr = r + p.r;
                            let nc = c + p.c;
                            if nr < 0 || nr >= h as i8 || nc < 0 || nc >= w as i8 {
                                valid = false;
                                break;
                            }
                            let idx = (nr as usize) * w + (nc as usize);
                            mask.set(idx, true);
                        }

                        if valid {
                            let anchor = (r as usize) * w + (c as usize);
                            shape_masks.push((anchor, mask));
                        }
                    }
                }
            }

            // Deduplicate masks (different rotations might produce identical footprints)
            shape_masks.sort_unstable_by(|a, b| a.1.cmp(&b.1));
            shape_masks.dedup_by(|a, b| a.1 == b.1);

            // Sort by anchor position for canonical ordering in the solver
            shape_masks.sort_by_key(|(anchor, _)| *anchor);

            if shape_masks.is_empty() {
                return None;
            }
            placements[id] = shape_masks;
        }

        Some(Self {
            placements,
            tasks,
            total_cells,
        })
    }

    fn solve(&self) -> bool {
        let mut grid = BitVec::<usize, Lsb0>::repeat(false, self.total_cells);
        self.backtrack(0, 0, 0, &mut grid)
    }

    fn backtrack(
        &self,
        task_idx: usize,
        count_placed: usize,
        min_anchor: usize,
        grid: &mut BitSlice<usize, Lsb0>,
    ) -> bool {
        // Base case: All tasks completed
        if task_idx >= self.tasks.len() {
            return true;
        }

        let (shape_id, total_needed) = self.tasks[task_idx];

        // If we finished placing the current shape type, move to the next one
        if count_placed >= total_needed {
            return self.backtrack(task_idx + 1, 0, 0, grid);
        }

        // Try to place the current shape
        let masks = &self.placements[shape_id];

        for (anchor, mask) in masks {
            // Enforce canonical ordering: identical shapes must be placed in increasing anchor order
            if *anchor < min_anchor {
                continue;
            }

            // Intersection check without allocation
            // Manually iterate bits to check for collision (AND)
            // `not_any()` is unavailable on intersection iterators in some versions, so we use `any` on zip
            let collision = grid.iter().zip(mask.iter()).any(|(g, m)| *g && *m);

            if !collision {
                // Place shape (XOR or OR works because we checked disjointness, XOR is often faster/reversible)
                // grid |= mask;
                // We do it manually to modify the slice in place efficiently
                let len = grid.len();
                // SAFETY: BitVecs are same length (total_cells)
                for i in 0..len {
                    if mask[i] {
                        grid.set(i, true);
                    }
                }

                // Recurse
                if self.backtrack(task_idx, count_placed + 1, *anchor, grid) {
                    return true;
                }

                // Backtrack (Remove shape)
                for i in 0..len {
                    if mask[i] {
                        grid.set(i, false);
                    }
                }
            }
        }

        false
    }
}

fn normalize(points: &mut [Point]) {
    if points.is_empty() {
        return;
    }
    points.sort();
    let origin = points[0];
    for p in points.iter_mut() {
        p.r -= origin.r;
        p.c -= origin.c;
    }
}

fn generate_variants(raw_points: &[Point]) -> Vec<Vec<Point>> {
    let mut unique = HashSet::new();
    let mut variants = Vec::new();
    let mut current = raw_points.to_vec();

    for i in 0..8 {
        for p in current.iter_mut() {
            let old_r = p.r;
            p.r = p.c;
            p.c = -old_r;
        }
        if i == 3 {
            for p in current.iter_mut() {
                p.c = -p.c;
            }
        }
        let mut norm = current.clone();
        normalize(&mut norm);
        if unique.insert(norm.clone()) {
            variants.push(norm);
        }
    }
    variants
}

fn parser<'a>() -> impl Parser<'a, &'a str, (Vec<Shape>, Vec<Region>), extra::Err<Rich<'a, char>>> {
    let newline = text::newline();
    let number = text::int(10).from_str::<usize>().unwrapped();

    // Suffix 1: Shape Definition ":\n###"
    let shape_suffix = just(':')
        .ignore_then(newline)
        .ignore_then(
            choice((just('#'), just('.')))
                .repeated()
                .at_least(1)
                .collect::<String>()
                .separated_by(newline)
                .collect::<Vec<String>>(),
        )
        .map(|lines| {
            let mut points = Vec::new();
            for (r, line) in lines.iter().enumerate() {
                for (c, char) in line.chars().enumerate() {
                    if char == '#' {
                        points.push(Point {
                            r: r as i8,
                            c: c as i8,
                        });
                    }
                }
            }
            normalize(&mut points);
            LineSuffix::Shape(points)
        });

    // Suffix 2: Region Definition "x5: 1 0..."
    let region_suffix = just('x')
        .ignore_then(number)
        .then_ignore(just(':').padded())
        .then(number.separated_by(just(' ')).collect())
        .map(|(height, reqs)| LineSuffix::Region(height, reqs));

    // Combine
    let line_parser = number
        .then(choice((shape_suffix, region_suffix)))
        .map(|(prefix, suffix)| match suffix {
            LineSuffix::Shape(points) => InputItem::Shape(Shape {
                id: prefix,
                area: points.len(),
                variants: generate_variants(&points),
            }),
            LineSuffix::Region(height, reqs) => InputItem::Region(Region {
                width: prefix,
                height,
                reqs,
            }),
        });

    line_parser
        .separated_by(newline.repeated().at_least(1))
        .allow_trailing()
        .collect::<Vec<InputItem>>()
        .map(|items| {
            let mut shapes = Vec::new();
            let mut regions = Vec::new();
            for item in items {
                match item {
                    InputItem::Shape(s) => {
                        let id = s.id;
                        if shapes.len() <= id {
                            shapes.resize(
                                id + 1,
                                Shape {
                                    id: 0,
                                    area: 0,
                                    variants: vec![],
                                },
                            );
                        }
                        shapes[id] = s;
                    }
                    InputItem::Region(r) => regions.push(r),
                }
            }
            (shapes, regions)
        })
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let (shapes, regions) = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let success_count = regions
        .par_iter()
        .map(|region| match Solver::new(&shapes, region) {
            Some(solver) => {
                if solver.solve() {
                    1
                } else {
                    0
                }
            }
            None => 0,
        })
        .sum::<usize>();

    Ok(success_count.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "0:
###
##.
##.

1:
###
##.
.##

2:
.##
###
##.

3:
##.
###
##.

4:
###
#..
###

5:
###
.#.
###

4x4: 0 0 0 0 2 0
12x5: 1 0 1 0 2 2
12x5: 1 0 1 0 3 2";
        assert_eq!("2", process(input)?);
        Ok(())
    }
}
