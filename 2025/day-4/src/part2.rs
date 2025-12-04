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

        // We manually count to avoid overhead of iterator chains in the hot loop
        let mut count = 0;
        for (dx, dy) in offsets {
            if self.get(x + dx, y + dy) {
                count += 1;
            }
        }
        count
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
            // Filter out empty rows (handles trailing newline at EOF)
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
    let mut grid = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let mut total_removed = 0;

    loop {
        let mut indices_to_remove = Vec::new();

        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = y * grid.width + x;

                // Only check cells that currently have paper
                if !grid.cells[idx] {
                    continue;
                }

                // Check condition: fewer than 4 adjacent paper rolls
                if grid.count_neighbors(x, y) < 4 {
                    indices_to_remove.push(idx);
                }
            }
        }

        if indices_to_remove.is_empty() {
            break;
        }
        total_removed += indices_to_remove.len();

        for idx in indices_to_remove {
            grid.cells[idx] = false;
        }
    }

    Ok(total_removed.to_string())
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
        assert_eq!("43", process(input)?);
        Ok(())
    }
}
