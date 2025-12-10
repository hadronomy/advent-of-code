use bitvec::prelude::*;
use chumsky::prelude::*;
use miette::*;

/// A bit vector backed by `usize` words with Least Significant Bit first ordering.
/// This aligns with standard CPU integer operations for maximum performance.
type Row = BitVec<usize, Lsb0>;

#[derive(Debug)]
struct Machine {
    /// Target configuration (b vector)
    target: Row,
    /// Button configurations (A matrix columns)
    buttons: Vec<Row>,
}

struct LinearSystem {
    /// Augmented matrix [A | b] in Reduced Row Echelon Form
    matrix: Vec<Row>,
    num_vars: usize,
    num_eqs: usize,
    /// Maps column index (variable) to pivot row index
    col_to_pivot: Vec<Option<usize>>,
    /// Indices of free variables (columns without pivots)
    free_vars: Vec<usize>,
}

impl LinearSystem {
    fn new(machine: &Machine) -> Self {
        let num_eqs = machine.target.len();
        let num_vars = machine.buttons.len();

        // Build Augmented Matrix [A | b]
        // Each inner BitVec is a row (equation).
        // Matrix dimensions: num_eqs x (num_vars + 1)
        let mut matrix = vec![Row::repeat(false, num_vars + 1); num_eqs];

        for (r, row) in matrix.iter_mut().enumerate() {
            // Fill A matrix part
            for (c, button) in machine.buttons.iter().enumerate() {
                // Safety: Parser ensures button length matches target length
                if unsafe { *button.get_unchecked(r) } {
                    row.set(c, true);
                }
            }
            // Fill b vector part (augmented column)
            if unsafe { *machine.target.get_unchecked(r) } {
                row.set(num_vars, true);
            }
        }

        Self {
            matrix,
            num_vars,
            num_eqs,
            col_to_pivot: vec![None; num_vars],
            free_vars: Vec::new(),
        }
    }

    /// Performs Gaussian Elimination to transform the matrix into Reduced Row Echelon Form (RREF).
    fn rref(&mut self) -> bool {
        let mut pivot_row = 0;

        for c in 0..self.num_vars {
            if pivot_row >= self.num_eqs {
                self.free_vars.push(c);
                continue;
            }

            // Find a row with a 1 in the current column (pivot)
            let mut pivot_found = None;
            for r in pivot_row..self.num_eqs {
                if self.matrix[r][c] {
                    pivot_found = Some(r);
                    break;
                }
            }

            if let Some(r) = pivot_found {
                self.matrix.swap(pivot_row, r);
                self.col_to_pivot[c] = Some(pivot_row);

                // Clone pivot row to avoid multiple mutable borrows
                let pivot_vec = self.matrix[pivot_row].clone();

                // XOR eliminate other rows (both below AND above for RREF)
                for i in 0..self.num_eqs {
                    if i != pivot_row && self.matrix[i][c] {
                        let row = &mut self.matrix[i];
                        *row ^= &pivot_vec;
                    }
                }
                pivot_row += 1;
            } else {
                self.free_vars.push(c);
            }
        }

        // Check for consistency: 0 = 1?
        // If a row is all zeros except the augmented column, no solution exists.
        for r in pivot_row..self.num_eqs {
            if self.matrix[r][self.num_vars] {
                return false;
            }
        }

        true
    }

    /// Extracts the particular solution and the basis of the null space.
    fn extract_solution_space(&self) -> (Row, Vec<Row>) {
        // Particular Solution (x_p)
        // Set all free variables to 0. Since matrix is in RREF,
        // the pivot variables simply take the value of the augmented column.
        let mut x_p = Row::repeat(false, self.num_vars);
        for (c, &pivot_row) in self.col_to_pivot.iter().enumerate() {
            if let Some(r) = pivot_row {
                if self.matrix[r][self.num_vars] {
                    x_p.set(c, true);
                }
            }
        }

        // Null Space Basis
        // For each free variable f, set x_f = 1, others = 0, and solve for pivots.
        let mut basis = Vec::with_capacity(self.free_vars.len());

        for &f in &self.free_vars {
            let mut v = Row::repeat(false, self.num_vars);
            v.set(f, true);

            // Back-substitute to find dependent pivot variables
            // Start from rightmost pivot columns to handle dependencies correctly
            for c in (0..f).rev() {
                if let Some(r) = self.col_to_pivot[c] {
                    // x_c = sum(A_ck * x_k) for k > c
                    // Check dot product of row `r` and current vector `v`
                    let mut dot = false;
                    for k in (c + 1)..self.num_vars {
                        if self.matrix[r][k] && v[k] {
                            dot = !dot;
                        }
                    }
                    if dot {
                        v.set(c, true);
                    }
                }
            }
            basis.push(v);
        }

        (x_p, basis)
    }

    /// Solves for the minimum Hamming weight (fewest button presses).
    /// Uses Gray Codes to iterate the null space efficiently.
    fn solve_min_weight(&mut self) -> Option<usize> {
        if !self.rref() {
            return None;
        }

        let (mut current_sol, null_basis) = self.extract_solution_space();
        let k = null_basis.len();

        // If no free variables, unique solution
        if k == 0 {
            return Some(current_sol.count_ones());
        }

        let mut min_weight = current_sol.count_ones();

        // Explicitly typed as usize to prevent "ambiguous numeric type" error
        let num_combinations: usize = 1 << k;

        // Gray Code Iteration:
        // iterate i from 1 to 2^k. The bit that changes between gray(i-1) and gray(i)
        // is the position of the lowest set bit in i (0-indexed).
        // This allows us to update the current solution with a single XOR.
        for i in 1..num_combinations {
            // Find index of the bit that flipped (trailing zeros of i)
            let basis_idx = i.trailing_zeros() as usize;

            // Update solution: x_new = x_old XOR basis[idx]
            current_sol ^= &null_basis[basis_idx];

            let weight = current_sol.count_ones();
            if weight < min_weight {
                min_weight = weight;
            }
        }

        Some(min_weight)
    }
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Machine>, extra::Err<Rich<'a, char>>> {
    // Custom whitespace parser that excludes newlines
    let hspace = any().filter(|c: &char| *c == ' ' || *c == '\t').repeated();

    let light = choice((just('.').to(false), just('#').to(true)));

    // [.##.]
    let diagram = light
        .repeated()
        .collect::<Vec<bool>>()
        .map(|v| v.into_iter().collect::<Row>())
        .delimited_by(just('['), just(']'));

    // (0,2,3)
    let indices = text::int(10)
        .from_str::<usize>()
        .unwrapped()
        .separated_by(just(','))
        .collect::<Vec<usize>>()
        .delimited_by(just('('), just(')'));

    // (0,2) (1,3) ...
    let buttons = indices.padded_by(hspace).repeated().collect::<Vec<_>>();

    // {3,5,4} (Ignored)
    let joltage = none_of("}")
        .repeated()
        .delimited_by(just('{'), just('}'))
        .ignored();

    let machine = diagram
        .then_ignore(hspace)
        .then(buttons)
        .then_ignore(joltage.or_not().padded_by(hspace))
        .map(|(target, raw_buttons)| {
            let len = target.len();
            let buttons = raw_buttons
                .into_iter()
                .map(|idxs| {
                    let mut row = Row::repeat(false, len);
                    for i in idxs {
                        if i < len {
                            row.set(i, true);
                        }
                    }
                    row
                })
                .collect();
            Machine { target, buttons }
        });

    machine
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let machines = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let total_presses: usize = machines
        .iter()
        .map(|m| {
            LinearSystem::new(m)
                .solve_min_weight()
                .expect("Machine configuration should be solvable")
        })
        .sum();

    Ok(total_presses.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "[.##.] (3) (1,3) (2) (2,3) (0,2) (0,1) {3,5,4,7}
[...#.] (0,2,3,4) (2,3) (0,4) (0,1,2) (1,2,3,4) {7,5,12,7,2}
[.###.#] (0,1,2,3,4) (0,3,4) (0,1,2,4,5) (1,2) {10,11,11,5,10,5}";
        assert_eq!("7", process(input)?);
        Ok(())
    }
}
