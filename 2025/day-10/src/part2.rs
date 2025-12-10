use chumsky::prelude::*;
use miette::{miette, Result};
use nalgebra::{DMatrix, DVector};
use rayon::prelude::*;

// -----------------------------------------------------------------------------
// Constants & Configuration
// -----------------------------------------------------------------------------

/// Numerical epsilon for comparing floating point values to zero.
const EPSILON: f64 = 1e-9;

/// Tolerance for Phase 1 feasibility check.
/// Relaxed to handle floating point drift in large numbers (10^13).
const PHASE1_TOLERANCE: f64 = 1e-4;

/// Tolerance for checking if a float represents an integer.
const INTEGRALITY_TOLERANCE: f64 = 1e-3;

/// Tolerance for pruning branches in B&B.
const PRUNING_TOLERANCE: f64 = 1e-5;

// -----------------------------------------------------------------------------
// Domain Models
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LinearSystem {
    pub a: DMatrix<f64>,
    pub b: DVector<f64>,
    pub c: DVector<f64>,
    /// Kept for final strict verification against float drift.
    pub original_b: DVector<f64>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub x: DVector<f64>,
    pub cost: f64,
}

// -----------------------------------------------------------------------------
// Simplex Solver Core
// -----------------------------------------------------------------------------

mod simplex {
    use super::*;

    /// Solves the Linear Programming relaxation of the system.
    pub fn solve(sys: &LinearSystem) -> Option<Solution> {
        // Phase 1: check feasibility and find initial BFS
        let (mut tableau, m, n) = setup_phase_one(sys);

        let phase1_obj_col = tableau.ncols() - 1;
        if !run_pivot_loop(&mut tableau, m, phase1_obj_col) {
            return None; // Unbounded (should not happen in Phase 1)
        }

        // Check Phase 1 objective (Minimize sum of artificials)
        let phase1_cost = tableau[(m, tableau.ncols() - 1)];
        if phase1_cost.abs() > PHASE1_TOLERANCE {
            return None; // Infeasible
        }

        // Phase 2: Optimize original objective
        let (mut phase2_tableau, active_rows) = prepare_phase_two(&tableau, m, n);
        setup_phase_two_objective(&mut phase2_tableau, &sys.c, active_rows, n);

        if !run_pivot_loop(&mut phase2_tableau, active_rows, n) {
            return None; // Unbounded
        }

        extract_solution(&phase2_tableau, active_rows, n)
    }

    fn setup_phase_one(sys: &LinearSystem) -> (DMatrix<f64>, usize, usize) {
        let m = sys.a.nrows();
        let n = sys.a.ncols();
        let width = n + m + 1; // Vars + Artificials + RHS
        let height = m + 1; // Constraints + Objective

        let mut tableau = DMatrix::zeros(height, width);

        // Setup constraints (handling negative RHS by flipping signs)
        for r in 0..m {
            let sign = if sys.b[r] < 0.0 { -1.0 } else { 1.0 };

            for c in 0..n {
                tableau[(r, c)] = sys.a[(r, c)] * sign;
            }
            tableau[(r, n + r)] = 1.0; // Artificial variable identity
            tableau[(r, width - 1)] = sys.b[r] * sign;
        }

        // Setup Phase 1 Objective: Maximize -Sum(Artificials)
        // Algebraically eliminate artificials from the objective row immediately.
        // Obj = -Sum(Row_i) for all i
        for c in 0..width {
            let col_sum: f64 = (0..m).map(|r| tableau[(r, c)]).sum();
            tableau[(m, c)] = -col_sum;
        }

        // Zero out the artificial columns in the objective row (canonical form)
        for i in 0..m {
            tableau[(m, n + i)] = 0.0;
        }

        (tableau, m, n)
    }

    fn prepare_phase_two(tableau: &DMatrix<f64>, m: usize, n: usize) -> (DMatrix<f64>, usize) {
        let width = tableau.ncols();
        // Identify which column is basic for each row
        let mut basis_col_for_row = vec![None; m];

        for r in 0..m {
            basis_col_for_row[r] = find_basis_col(tableau, r, m, width - 1);
        }

        // Basis Repair: If Artificial variable is basic, try to pivot it out
        let mut repaired_tableau = tableau.clone();
        for r in 0..m {
            if let Some(bc) = basis_col_for_row[r] {
                if bc >= n {
                    // Artificial is basic. Try to find a non-artificial pivot.
                    if let Some(pc) = (0..n).find(|&c| repaired_tableau[(r, c)].abs() > EPSILON) {
                        pivot(&mut repaired_tableau, r, pc, m, width - 1);
                        basis_col_for_row[r] = Some(pc);
                    } else {
                        // Row is 0=0 (redundant). Mark for removal.
                        basis_col_for_row[r] = None;
                    }
                }
            }
        }

        // Filter out redundant rows
        let active_row_indices: Vec<usize> =
            (0..m).filter(|&r| basis_col_for_row[r].is_some()).collect();

        let new_m = active_row_indices.len();
        let mut phase2 = DMatrix::zeros(new_m + 1, n + 1);

        for (new_r, &old_r) in active_row_indices.iter().enumerate() {
            for c in 0..n {
                phase2[(new_r, c)] = repaired_tableau[(old_r, c)];
            }
            phase2[(new_r, n)] = repaired_tableau[(old_r, width - 1)]; // Copy RHS
        }

        (phase2, new_m)
    }

    fn setup_phase_two_objective(
        phase2: &mut DMatrix<f64>,
        c_vec: &DVector<f64>,
        m: usize,
        n: usize,
    ) {
        // Start with original costs
        for c in 0..n {
            phase2[(m, c)] = c_vec[c];
        }

        // Canonicalize: Eliminate basic variables from objective row
        for r in 0..m {
            // Find the basic column in this row (it will be a unit vector)
            if let Some(bc) = find_basis_col(phase2, r, m, n) {
                let factor = phase2[(m, bc)];
                if factor.abs() > EPSILON {
                    for c in 0..=n {
                        phase2[(m, c)] -= factor * phase2[(r, c)];
                    }
                }
            }
        }
    }

    fn extract_solution(tableau: &DMatrix<f64>, m: usize, n: usize) -> Option<Solution> {
        let mut x = DVector::zeros(n);

        for c in 0..n {
            // Check if column c is basic
            let mut basic_row = None;
            let mut non_zeros = 0;

            for r in 0..m {
                let val = tableau[(r, c)];
                if val.abs() > EPSILON {
                    non_zeros += 1;
                    if (val - 1.0).abs() < EPSILON {
                        basic_row = Some(r);
                    }
                }
            }

            if non_zeros == 1 {
                if let Some(r) = basic_row {
                    x[c] = tableau[(r, n)];
                }
            }
        }

        Some(Solution {
            x,
            cost: -tableau[(m, n)], // Objective maximization adjustment
        })
    }

    fn pivot(mat: &mut DMatrix<f64>, pr: usize, pc: usize, m: usize, n: usize) {
        let pivot_val = mat[(pr, pc)];
        let inv = 1.0 / pivot_val;

        // Normalize pivot row
        for c in 0..=n {
            mat[(pr, c)] *= inv;
        }

        // Eliminate other rows
        for r in 0..=m {
            if r != pr {
                let factor = mat[(r, pc)];
                if factor.abs() > EPSILON {
                    for c in 0..=n {
                        mat[(r, c)] -= factor * mat[(pr, c)];
                    }
                }
            }
        }
    }

    fn run_pivot_loop(mat: &mut DMatrix<f64>, m: usize, n: usize) -> bool {
        let max_iters = 5000;

        for _ in 0..max_iters {
            // Bland's Rule: First column with negative reduced cost
            let pivot_col = (0..n).find(|&c| mat[(m, c)] < -EPSILON);

            match pivot_col {
                None => return true, // Optimal
                Some(pc) => {
                    // Min Ratio Test
                    let mut pivot_row = None;
                    let mut min_ratio = f64::MAX;

                    for r in 0..m {
                        let val = mat[(r, pc)];
                        if val > EPSILON {
                            let ratio = mat[(r, n)] / val;
                            if ratio < min_ratio {
                                min_ratio = ratio;
                                pivot_row = Some(r);
                            }
                        }
                    }

                    match pivot_row {
                        None => return false, // Unbounded
                        Some(pr) => pivot(mat, pr, pc, m, n),
                    }
                }
            }
        }
        false // Iteration limit exceeded
    }

    fn find_basis_col(mat: &DMatrix<f64>, r: usize, m: usize, total_cols: usize) -> Option<usize> {
        for c in 0..total_cols {
            // Look for 1.0
            if (mat[(r, c)] - 1.0).abs() < EPSILON {
                // Ensure it's a unit vector (zeros elsewhere)
                let is_unit =
                    (0..m).all(|other_r| other_r == r || mat[(other_r, c)].abs() < EPSILON);
                if is_unit {
                    return Some(c);
                }
            }
        }
        None
    }
}

// -----------------------------------------------------------------------------
// Mixed Integer Linear Programming (Branch & Bound)
// -----------------------------------------------------------------------------

mod milp {
    use super::*;

    struct BranchNode {
        lower_bounds: Vec<f64>,
        upper_bounds: Vec<Option<f64>>,
    }

    pub fn solve(sys: &LinearSystem) -> Option<usize> {
        let n = sys.a.ncols();
        let mut best_int_cost = f64::MAX;

        // Explicitly annotate type for the compiler
        let mut best_sol: Option<Vec<usize>> = None;

        let mut stack = vec![BranchNode {
            lower_bounds: vec![0.0; n],
            upper_bounds: vec![None; n],
        }];

        while let Some(node) = stack.pop() {
            // Construct the relaxed LP system for this node
            let (lp_sys, shift_cost) = match build_relaxed_system(sys, &node) {
                Some(res) => res,
                None => continue, // Infeasible bounds
            };

            // Solve Relaxed LP
            if let Some(sol) = simplex::solve(&lp_sys) {
                let total_cost = sol.cost + shift_cost;

                // Pruning: Bound check
                if total_cost >= best_int_cost - PRUNING_TOLERANCE {
                    continue;
                }

                // Check Integrality
                let (full_x, first_fractional) = map_solution_to_original(&sol, &node);

                if let Some((idx, val)) = first_fractional {
                    // Branching: Split on the fractional variable
                    let floor_val = val.floor();
                    let ceil_val = val.ceil();

                    // Branch 1: x <= floor
                    let mut left = BranchNode {
                        lower_bounds: node.lower_bounds.clone(),
                        upper_bounds: node.upper_bounds.clone(),
                    };
                    let current_ub = left.upper_bounds[idx].unwrap_or(f64::MAX);
                    left.upper_bounds[idx] = Some(current_ub.min(floor_val));

                    // Branch 2: x >= ceil
                    let mut right = BranchNode {
                        lower_bounds: node.lower_bounds.clone(),
                        upper_bounds: node.upper_bounds.clone(),
                    };
                    right.lower_bounds[idx] = right.lower_bounds[idx].max(ceil_val);

                    stack.push(left);
                    stack.push(right);
                } else {
                    // Integer Solution Found
                    if verify_strict(sys, &full_x) {
                        let cost: usize = full_x.iter().map(|&x| x.round() as usize).sum();
                        if (cost as f64) < best_int_cost {
                            best_int_cost = cost as f64;
                            best_sol = Some(full_x.iter().map(|&x| x.round() as usize).collect());
                        }
                    }
                }
            }
        }

        best_sol.map(|s| s.iter().sum())
    }

    fn build_relaxed_system(sys: &LinearSystem, node: &BranchNode) -> Option<(LinearSystem, f64)> {
        let mut work_sys = sys.clone();
        let mut shift_cost = 0.0;
        let n = sys.a.ncols();

        // Apply Lower Bounds: Shift RHS (b' = b - A * lb)
        for c in 0..n {
            let lb = node.lower_bounds[c];
            if lb > 0.0 {
                let col_vec = work_sys.a.column(c);
                work_sys.b -= col_vec * lb;
                shift_cost += lb * sys.c[c];
            }
        }

        // Apply Upper Bounds: Add slack constraints (x_shifted + slack = UB - LB)
        let mut slack_constraints = Vec::new();
        for c in 0..n {
            if let Some(ub) = node.upper_bounds[c] {
                let limit = ub - node.lower_bounds[c];
                // Check feasibility allowing for tiny float error
                if limit < -1e-3 {
                    return None;
                }
                slack_constraints.push((c, limit.max(0.0)));
            }
        }

        if !slack_constraints.is_empty() {
            let added_rows = slack_constraints.len();
            let old_m = work_sys.a.nrows();
            let old_n = work_sys.a.ncols();

            // Resize matrices
            work_sys.a = work_sys.a.resize_vertically(old_m + added_rows, 0.0); // Adds 0 rows
            work_sys.a = work_sys.a.resize_horizontally(old_n + added_rows, 0.0); // Adds 0 cols
            work_sys.b = work_sys.b.resize_vertically(old_m + added_rows, 0.0);
            work_sys.c = work_sys.c.resize_vertically(old_n + added_rows, 0.0);

            for (i, &(var_idx, limit)) in slack_constraints.iter().enumerate() {
                let r = old_m + i;
                let s = old_n + i; // Slack column index

                work_sys.a[(r, var_idx)] = 1.0;
                work_sys.a[(r, s)] = 1.0;
                work_sys.b[r] = limit;
            }
        }

        Some((work_sys, shift_cost))
    }

    fn map_solution_to_original(
        sol: &Solution,
        node: &BranchNode,
    ) -> (Vec<f64>, Option<(usize, f64)>) {
        let n = node.lower_bounds.len();
        let mut full_x = vec![0.0; n];
        let mut first_fractional = None;

        for c in 0..n {
            let val = sol.x[c] + node.lower_bounds[c];
            full_x[c] = val;

            // Only check fractional if we haven't found one yet
            if first_fractional.is_none() {
                let rounded = val.round();
                if (val - rounded).abs() > INTEGRALITY_TOLERANCE {
                    first_fractional = Some((c, val));
                }
            }
        }
        (full_x, first_fractional)
    }

    fn verify_strict(sys: &LinearSystem, x: &[f64]) -> bool {
        let m = sys.original_b.len();
        let n = x.len();

        for r in 0..m {
            let lhs: f64 = (0..n).map(|c| sys.a[(r, c)] * x[c].round()).sum();
            // Loose verification for 10^13 magnitude inputs
            if (lhs - sys.original_b[r]).abs() > 0.5 {
                return false;
            }
        }
        true
    }
}

// -----------------------------------------------------------------------------
// Parsing & Entry Point
// -----------------------------------------------------------------------------

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<LinearSystem>, extra::Err<Rich<'a, char>>> {
    let hspace = one_of(" \t").repeated();

    // Example: [...] (A) (B) {Targets}
    let diagram = none_of("]")
        .repeated()
        .delimited_by(just('['), just(']'))
        .ignored();

    let num_list = text::int(10)
        .from_str::<f64>()
        .unwrapped()
        .separated_by(just(','))
        .collect::<Vec<f64>>();

    let buttons = num_list
        .delimited_by(just('('), just(')'))
        .map(|v| v.into_iter().map(|x| x as usize).collect::<Vec<_>>());

    let targets = num_list.delimited_by(just('{'), just('}'));

    let machine = diagram
        .then_ignore(hspace)
        .ignore_then(buttons.padded_by(hspace).repeated().collect::<Vec<_>>())
        .then(targets)
        .map(|(buttons, targets)| {
            let m = targets.len();
            let n = buttons.len();

            let mut a_mat = DMatrix::zeros(m, n);
            let mut b_vec = DVector::zeros(m);
            let c_vec = DVector::from_element(n, 1.0); // Cost = 1 per press

            for (col, rows) in buttons.into_iter().enumerate() {
                for row in rows {
                    if row < m {
                        a_mat[(row, col)] = 1.0;
                    }
                }
            }
            for (row, &val) in targets.iter().enumerate() {
                if row < m {
                    b_vec[row] = val;
                }
            }

            LinearSystem {
                a: a_mat,
                b: b_vec.clone(),
                c: c_vec,
                original_b: b_vec,
            }
        });

    machine
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

pub fn process(input: &str) -> Result<String> {
    let systems = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let total: usize = systems
        .par_iter()
        .map(|sys| milp::solve(sys).unwrap_or(0))
        .sum();

    Ok(total.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "[.##.] (3) (1,3) (2) (2,3) (0,2) (0,1) {3,5,4,7}
[...#.] (0,2,3,4) (2,3) (0,4) (0,1,2) (1,2,3,4) {7,5,12,7,2}
[.###.#] (0,1,2,3,4) (0,3,4) (0,1,2,4,5) (1,2) {10,11,11,5,10,5}";
        assert_eq!("33", process(input)?);
        Ok(())
    }
}
