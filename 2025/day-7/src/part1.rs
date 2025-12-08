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
                        tiles.push(Tile::Empty); // S behaves like empty space for physics
                    }
                    '^' => tiles.push(Tile::Splitter),
                    // Treat anything else ('.') as empty
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

    // We only need to track which columns have a beam in the current row.
    // Using a boolean vector implicitly handles beam merging.
    let mut current_beams = vec![false; grid.width];
    let mut next_beams = vec![false; grid.width];

    // Initialize the beam at S
    current_beams[sx] = true;

    let mut total_splits = 0;

    // Simulate row by row, starting from the source row
    for y in sy..grid.height {
        // Clear the next row buffer
        next_beams.fill(false);

        let mut active_beams_count = 0;

        for x in 0..grid.width {
            if current_beams[x] {
                active_beams_count += 1;
                let idx = y * grid.width + x;

                match grid.tiles[idx] {
                    Tile::Empty => {
                        // Beam continues straight down
                        // It will exist at column x in row y+1
                        next_beams[x] = true;
                    }
                    Tile::Splitter => {
                        // Beam hits splitter
                        total_splits += 1;

                        // Beam stops, new beams emitted left (x-1) and right (x+1)
                        // These new beams will exist in row y+1
                        if x > 0 {
                            next_beams[x - 1] = true;
                        }
                        if x + 1 < grid.width {
                            next_beams[x + 1] = true;
                        }
                    }
                }
            }
        }

        if active_beams_count == 0 {
            break;
        }

        // Advance to the next row
        // We swap the buffers so `next_beams` becomes `current_beams` for the next iteration
        std::mem::swap(&mut current_beams, &mut next_beams);
    }

    Ok(total_splits.to_string())
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
        assert_eq!("21", process(input)?);
        Ok(())
    }
}
