use miette::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tile {
    Empty,
    Splitter,
}

struct Grid {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    start: (usize, usize),
}

impl Grid {
    fn from_str(input: &str) -> Result<Self> {
        let mut tiles = Vec::new();
        let mut start = None;
        let mut width = 0;
        let mut height = 0;

        for (y, line) in input.lines().enumerate() {
            width = line.len();
            height += 1;
            for (x, c) in line.chars().enumerate() {
                match c {
                    'S' => {
                        start = Some((x, y));
                        tiles.push(Tile::Empty);
                    }
                    '^' => tiles.push(Tile::Splitter),
                    _ => tiles.push(Tile::Empty),
                }
            }
        }

        let start = start.ok_or(miette!("No start position 'S' found in grid"))?;

        Ok(Grid {
            width,
            height,
            tiles,
            start,
        })
    }
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let grid = Grid::from_str(input)?;
    let (sx, sy) = grid.start;

    // We track the number of distinct timelines (paths) reaching each column.
    // u128 is used because splitters cause exponential growth (2^N).
    let mut current_counts: Vec<u128> = vec![0; grid.width];
    let mut next_counts: Vec<u128> = vec![0; grid.width];

    // Initialize: 1 particle timeline starts at S
    current_counts[sx] = 1;

    // Accumulator for timelines that exit the grid boundaries (sides or bottom)
    let mut finished_timelines: u128 = 0;

    for y in sy..grid.height {
        // Clear next row buffer
        next_counts.fill(0);

        let mut active = false;

        for x in 0..grid.width {
            let count = current_counts[x];
            if count == 0 {
                continue;
            }
            active = true;

            let idx = y * grid.width + x;
            match grid.tiles[idx] {
                Tile::Empty => {
                    // Beam passes straight through to the next row
                    next_counts[x] += count;
                }
                Tile::Splitter => {
                    // Beam splits: 1 path becomes 2 distinct paths (Left and Right)

                    // Left Branch
                    if x > 0 {
                        next_counts[x - 1] += count;
                    } else {
                        // Exited grid to the left
                        finished_timelines += count;
                    }

                    // Right Branch
                    if x + 1 < grid.width {
                        next_counts[x + 1] += count;
                    } else {
                        // Exited grid to the right
                        finished_timelines += count;
                    }
                }
            }
        }

        if !active {
            break;
        }

        // Move to the next row
        std::mem::swap(&mut current_counts, &mut next_counts);
    }

    // Add all timelines that successfully reached the bottom of the grid
    finished_timelines += current_counts.iter().sum::<u128>();

    Ok(finished_timelines.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = ".......S.......
...............
.......^.......
...............
......^.^......
...............
.....^.^.^.....
...............
....^.^...^....
...............
...^.^...^.^...
...............
..^...^.....^..
...............
.^.^.^.^.^...^.
...............";
        assert_eq!("40", process(input)?);
        Ok(())
    }
}
