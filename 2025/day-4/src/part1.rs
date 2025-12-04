use chumsky::prelude::*;
use miette::*;

struct Grid {
    width: usize,
    height: usize,
    // true = '@' (paper), false = '.' (empty)
    cells: Vec<bool>,
}

impl Grid {
    /// Returns the value at (x, y), or false if out of bounds.
    fn get(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let x = x as usize;
        let y = y as usize;
        // Bounds check ensures we don't access indices beyond the grid logic,
        // but we must ensure cells.len() == width * height for this to be safe.
        if x >= self.width || y >= self.height {
            return false;
        }
        self.cells[y * self.width + x]
    }

    /// Counts how many neighbors (including diagonals) contain paper.
    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let x = x as isize;
        let y = y as isize;
        let offsets = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        offsets
            .iter()
            .filter(|&&(dx, dy)| self.get(x + dx, y + dy))
            .count()
    }
}

/// Parses the grid of characters into a Grid struct.
fn parser<'a>() -> impl Parser<'a, &'a str, Grid, extra::Err<Rich<'a, char>>> {
    let cell = just('@').to(true).or(just('.').to(false));

    cell.repeated()
        .collect::<Vec<_>>()
        .separated_by(text::newline())
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(|rows| {
            // Filter out empty rows to prevent "ragged" grids caused by trailing newlines
            let rows: Vec<_> = rows.into_iter().filter(|r| !r.is_empty()).collect();

            let height = rows.len();
            let width = rows.first().map(|r| r.len()).unwrap_or(0);
            let cells = rows.into_iter().flatten().collect();

            Grid {
                width,
                height,
                cells,
            }
        })
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let grid = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let mut accessible_count = 0;

    for y in 0..grid.height {
        for x in 0..grid.width {
            // First check if there is actually a roll here.
            // Direct access is safe here because we loop strictly within bounds
            // and we ensured the grid is built correctly.
            if !grid.cells[y * grid.width + x] {
                continue;
            }

            // Check neighbor condition
            if grid.count_neighbors(x, y) < 4 {
                accessible_count += 1;
            }
        }
    }

    Ok(accessible_count.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "..@@.@@@@.
@@@.@.@.@@
@@@@@.@.@@
@.@@@@..@.
@@.@@@@.@@
.@@@@@@@.@
.@.@.@.@@@
@.@@@.@@@@
.@@@@@@@@.
@.@.@@@.@.";
        assert_eq!("13", process(input)?);
        Ok(())
    }
}
