#![allow(dead_code)]

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
        self.starts
            .binary_search(&val)
            .expect("Coordinate not found in map")
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
        let visited = Self::scanline_flood_fill(&boundaries, width, height);
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

            // Calculate ranges
            let x_start = x1.min(x2);
            let x_end = x1.max(x2);
            let y_start = y1.min(y2);
            let y_end = y1.max(y2);

            for y in y_start..=y_end {
                let row_offset = y * width;
                let start = row_offset + x_start;
                // slice range is exclusive at the end, so +1
                let end = row_offset + x_end + 1;
                grid[start..end].fill(true);
            }
        }
        grid
    }

    fn scanline_flood_fill(
        boundaries: &BitSlice<u64, Lsb0>,
        width: usize,
        height: usize,
    ) -> BitVec<u64, Lsb0> {
        let mut visited = bitvec![u64, Lsb0; 0; width * height];
        let mut stack = Vec::with_capacity(height * 4);

        // Start at (0,0) - guaranteed outside due to padding
        stack.push((0usize, 0usize));

        while let Some((x, y)) = stack.pop() {
            let row_offset = y * width;
            let idx = row_offset + x;

            if visited[idx] {
                continue;
            }

            // Scan Left
            let mut lx = x;
            while lx > 0 {
                let prev = lx - 1;
                let p_idx = row_offset + prev;
                // Check boundary logic without recursion
                if !boundaries[p_idx] && !visited[p_idx] {
                    lx = prev;
                } else {
                    break;
                }
            }

            // Scan Right
            let mut rx = x;
            while rx < width - 1 {
                let next = rx + 1;
                let n_idx = row_offset + next;
                if !boundaries[n_idx] && !visited[n_idx] {
                    rx = next;
                } else {
                    break;
                }
            }

            // Block fill the visited row segment
            visited[row_offset + lx..=row_offset + rx].fill(true);

            // Scan Rows Above and Below to seed new spans
            let scan_row = |stack: &mut Vec<(usize, usize)>, ny: usize| {
                let n_row_offset = ny * width;
                let mut i = lx;
                while i <= rx {
                    let idx = n_row_offset + i;
                    if !boundaries[idx] && !visited[idx] {
                        stack.push((i, ny));
                        // Skip the contiguous segment we just pushed to avoid duplicate seeds
                        while i <= rx && !boundaries[n_row_offset + i] && !visited[n_row_offset + i]
                        {
                            i += 1;
                        }
                    }
                    i += 1;
                }
            };

            if y > 0 {
                scan_row(&mut stack, y - 1);
            }
            if y < height - 1 {
                scan_row(&mut stack, y + 1);
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
        let pw = width + 1;
        let ph = height + 1;
        let mut data = vec![0u64; pw * ph];

        // Flatten lengths to avoid double indirection/bounds checking in hot loop
        let x_lengths: Vec<u64> = x_map.lengths.iter().map(|&x| x as u64).collect();

        let mut visited_iter = visited_exterior.iter();

        for y in 0..height {
            // Safety: loop is bounded by height
            let y_len = unsafe { *y_map.lengths.get_unchecked(y) } as u64;

            // Pre-calculate row pointers
            let row_prev_start = y * pw;
            let row_curr_start = (y + 1) * pw;

            let mut row_acc = 0;

            for x in 0..width {
                // Safety: iterator is exactly width * height long
                let is_exterior = visited_iter.next().unwrap();
                let is_interior_mask = (!is_exterior) as u64;

                // Safety: x bounded by width
                let x_len = unsafe { *x_lengths.get_unchecked(x) };

                row_acc += x_len * y_len * is_interior_mask;

                // Safety: calculated indices are within allocated vector
                unsafe {
                    let top = *data.get_unchecked(row_prev_start + x + 1);
                    *data.get_unchecked_mut(row_curr_start + x + 1) = top + row_acc;
                }
            }
        }

        Grid2D {
            width: pw,
            height: ph,
            data,
        }
    }

    /// Queries valid area.
    /// Uses unchecked access because indices are derived from AxisMap which guarantees validity.
    #[inline(always)]
    fn query_area(&self, x_range: Range<usize>, y_range: Range<usize>) -> u64 {
        let pw = self.prefix_area.width;

        // Map compressed map indices to 1-based prefix array indices
        // Range from main loop is (min..max inclusive).
        let idx_x_high = x_range.end + 1;
        let idx_x_low = x_range.start;
        let idx_y_high = y_range.end + 1;
        let idx_y_low = y_range.start;

        let row_high_offset = idx_y_high * pw;
        let row_low_offset = idx_y_low * pw;

        unsafe {
            let a = *self
                .prefix_area
                .data
                .get_unchecked(row_high_offset + idx_x_high);
            let b = *self
                .prefix_area
                .data
                .get_unchecked(row_high_offset + idx_x_low);
            let c = *self
                .prefix_area
                .data
                .get_unchecked(row_low_offset + idx_x_high);
            let d = *self
                .prefix_area
                .data
                .get_unchecked(row_low_offset + idx_x_low);

            // (A + D) - B - C
            (a + d).wrapping_sub(b).wrapping_sub(c)
        }
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

    // Pre-calculate indices
    let x_map = AxisMap::new(points.iter().map(|p| p.x));
    let y_map = AxisMap::new(points.iter().map(|p| p.y));

    // Combine Point and Map Indices into one struct to improve cache locality
    // and reduce lookups in the parallel loop.
    let indexed_points: Vec<(Point, (usize, usize))> = points
        .iter()
        .map(|&p| (p, (x_map.index_of(p.x), y_map.index_of(p.y))))
        .collect();

    let max_valid_area = indexed_points
        .par_iter()
        .enumerate()
        .map(|(i, (p1, (x1, y1)))| {
            let mut local_max = 0;

            for (p2, (x2, y2)) in indexed_points.iter().skip(i + 1) {
                // Computes min/max for X and Y simultaneously
                let min_p = p1.min(*p2);
                let max_p = p1.max(*p2);

                // Calculate geometric area using vector operations
                // (max - min).abs() + 1
                let dims = (max_p - min_p).abs() + 1;
                let geometric_area = (dims.x as u64) * (dims.y as u64);

                // Integer comparisons are very fast
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
