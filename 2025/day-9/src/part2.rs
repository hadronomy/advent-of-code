use bitvec::prelude::*;
use chumsky::prelude::*;
use glam::I64Vec2;
use miette::*;
use rayon::prelude::*;
use std::ops::Range;


type Point = I64Vec2;

/// A dense 2D grid wrapper for flattened vectors.
#[derive(Debug, Clone)]
struct Grid2D<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T: Clone + Default> Grid2D<T> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![T::default(); width * height],
        }
    }

    #[inline(always)]
    fn get(&self, x: usize, y: usize) -> Option<&T> {
        if x >= self.width || y >= self.height {
            None
        } else {
            // Safety: Bounds checked above
            Some(&self.data[y * self.width + x])
        }
    }

    #[inline(always)]
    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(&mut self.data[y * self.width + x])
        }
    }
}

#[derive(Debug, Clone)]
struct AxisMap {
    starts: Vec<i64>,
    lengths: Vec<i64>,
}

impl AxisMap {
    fn new(coords: impl Iterator<Item = i64>) -> Self {
        let mut unique: Vec<i64> = coords.collect();
        
        // Add padding to guarantee an outer boundary for flood fill
        if let (Some(&min), Some(&max)) = (unique.iter().min(), unique.iter().max()) {
            unique.push(min - 1);
            unique.push(max + 1);
        }
        
        unique.sort_unstable();
        unique.dedup();

        let mut starts = Vec::with_capacity(unique.len() * 2);
        let mut lengths = Vec::with_capacity(unique.len() * 2);

        for (i, &curr) in unique.iter().enumerate() {
            starts.push(curr);
            lengths.push(1);

            if let Some(&next) = unique.get(i + 1) {
                if next > curr + 1 {
                    starts.push(curr + 1);
                    lengths.push(next - curr - 1);
                }
            }
        }

        Self { starts, lengths }
    }

    #[inline]
    fn index_of(&self, val: i64) -> usize {
        self.starts.binary_search(&val).expect("Coordinate not found in map")
    }

    fn size(&self) -> usize {
        self.starts.len()
    }
}

// -----------------------------------------------------------------------------
// Geometry Engine
// -----------------------------------------------------------------------------

struct GeometryEngine {
    prefix_area: Grid2D<u64>,
}

impl GeometryEngine {
    fn build(points: &[Point]) -> Self {
        let x_map = AxisMap::new(points.iter().map(|p| p.x));
        let y_map = AxisMap::new(points.iter().map(|p| p.y));
        let width = x_map.size();
        let height = y_map.size();

        let boundaries = Self::mark_boundaries(points, &x_map, &y_map, width, height);
        let visited = Self::flood_fill_exterior(&boundaries, width, height);
        let prefix_area = Self::compute_prefix_sums(&visited, &x_map, &y_map, width, height);

        Self { prefix_area }
    }

    fn mark_boundaries(
        points: &[Point],
        x_map: &AxisMap,
        y_map: &AxisMap,
        width: usize,
        height: usize,
    ) -> BitVec<u64, Lsb0> {
        let mut grid = bitvec![u64, Lsb0; 0; width * height];
        
        let mapped_points = points
            .iter()
            .map(|p| (x_map.index_of(p.x), y_map.index_of(p.y)))
            .collect::<Vec<_>>();

        for i in 0..mapped_points.len() {
            let (x1, y1) = mapped_points[i];
            let (x2, y2) = mapped_points[(i + 1) % mapped_points.len()];

            let x_range = x1.min(x2)..=x1.max(x2);
            let y_range = y1.min(y2)..=y1.max(y2);

            for y in y_range {
                for x in x_range.clone() {
                    grid.set(y * width + x, true);
                }
            }
        }
        grid
    }

    fn flood_fill_exterior(
        boundaries: &BitSlice<u64, Lsb0>,
        width: usize,
        height: usize,
    ) -> BitVec<u64, Lsb0> {
        let mut visited = bitvec![u64, Lsb0; 0; width * height];
        let mut stack: Vec<(usize, usize)> = Vec::with_capacity(width + height);
        
        // Start at (0,0) - guaranteed outside due to padding
        stack.push((0, 0));
        visited.set(0, true);

        while let Some((cx, cy)) = stack.pop() {
            // Check 4 neighbors
            let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)];

            for (dx, dy) in offsets {
                let nx = cx as isize + dx;
                let ny = cy as isize + dy;

                if nx < 0 || nx >= width as isize || ny < 0 || ny >= height as isize {
                    continue;
                }

                let nx = nx as usize;
                let ny = ny as usize;
                let idx = ny * width + nx;

                if !visited[idx] && !boundaries[idx] {
                    visited.set(idx, true);
                    stack.push((nx, ny));
                }
            }
        }
        visited
    }

    fn compute_prefix_sums(
        visited_exterior: &BitSlice<u64, Lsb0>,
        x_map: &AxisMap,
        y_map: &AxisMap,
        width: usize,
        height: usize,
    ) -> Grid2D<u64> {
        let mut prefix = Grid2D::new(width + 1, height + 1);

        for y in 0..height {
            for x in 0..width {
                // A tile is valid (Interior or Boundary) if NOT visited by exterior flood fill
                let is_valid = !visited_exterior[y * width + x];
                
                let area = if is_valid {
                    (x_map.lengths[x] * y_map.lengths[y]) as u64
                } else {
                    0
                };

                let sum = area 
                    + prefix.get(x + 1, y).unwrap_or(&0)      // top
                    + prefix.get(x, y + 1).unwrap_or(&0)      // left
                    - prefix.get(x, y).unwrap_or(&0);         // diag

                if let Some(cell) = prefix.get_mut(x + 1, y + 1) {
                    *cell = sum;
                }
            }
        }
        prefix
    }

    /// Queries valid area in compressed index range [x1, x2] x [y1, y2] inclusive.
    fn query_area(&self, x_range: Range<usize>, y_range: Range<usize>) -> u64 {
        let (x1, x2) = (x_range.start, x_range.end);
        let (y1, y2) = (y_range.start, y_range.end);

        // Map to 1-based prefix array indices
        let x_high = x2 + 1;
        let x_low = x1;
        let y_high = y2 + 1;
        let y_low = y1;

        let a = self.prefix_area.get(x_high, y_high).copied().unwrap_or(0);
        let b = self.prefix_area.get(x_high, y_low).copied().unwrap_or(0);
        let c = self.prefix_area.get(x_low, y_high).copied().unwrap_or(0);
        let d = self.prefix_area.get(x_low, y_low).copied().unwrap_or(0);

        // (Total Rect) - (Top Strip) - (Left Strip) + (Top-Left Overlap)
        // Order operations to prevent unsigned underflow: (A + D) - B - C
        (a + d).wrapping_sub(b).wrapping_sub(c)
    }
}


fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Point>, extra::Err<Rich<'a, char>>> {
    let coord = text::int(10).from_str::<i64>().unwrapped();
    coord
        .then_ignore(just(','))
        .then(coord)
        .map(|(x, y)| Point::new(x, y))
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let points = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    if points.len() < 2 {
        return Ok("0".to_string());
    }

    let engine = GeometryEngine::build(&points);

    // Pre-calculate indices for all points to avoid lookups in the hot loop
    let x_map = AxisMap::new(points.iter().map(|p| p.x));
    let y_map = AxisMap::new(points.iter().map(|p| p.y));
    
    let indexed_points: Vec<(Point, (usize, usize))> = points.iter()
        .map(|&p| (p, (x_map.index_of(p.x), y_map.index_of(p.y))))
        .collect();

    // Parallel check of all pairs
    let max_valid_area = indexed_points
        .par_iter()
        .enumerate()
        .map(|(i, (p1, (x1, y1)))| {
            let mut local_max = 0;
            
            for (p2, (x2, y2)) in indexed_points.iter().skip(i + 1) {
                let min_x = p1.x.min(p2.x);
                let max_x = p1.x.max(p2.x);
                let min_y = p1.y.min(p2.y);
                let max_y = p1.y.max(p2.y);

                let geometric_area = ((max_x - min_x).unsigned_abs() + 1) * ((max_y - min_y).unsigned_abs() + 1);
                
                let idx_x1 = *x1.min(x2);
                let idx_x2 = *x1.max(x2);
                let idx_y1 = *y1.min(y2);
                let idx_y2 = *y1.max(y2);

                let valid_area = engine.query_area(idx_x1..idx_x2, idx_y1..idx_y2);

                if valid_area == geometric_area {
                    local_max = local_max.max(valid_area);
                }
            }
            local_max
        })
        .max()
        .unwrap_or(0);

    Ok(max_valid_area.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "7,1
11,1
11,7
9,7
9,5
2,5
2,3
7,3";
        assert_eq!("24", process(input)?);
        Ok(())
    }
}