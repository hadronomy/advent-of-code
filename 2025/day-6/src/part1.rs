use chumsky::prelude::*;
use itertools::Itertools;
use miette::*;

#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Mul,
}

#[derive(Debug)]
struct Problem {
    numbers: Vec<u64>,
    op: Op,
}

impl Problem {
    fn solve(&self) -> u64 {
        match self.op {
            Op::Add => self.numbers.iter().sum(),
            Op::Mul => self.numbers.iter().product(),
        }
    }
}

#[derive(Clone, Debug)]
enum Token {
    Num(u64),
    Op(Op),
}

/// Parser for a single line content within a problem block.
fn line_content_parser<'a>() -> impl Parser<'a, &'a str, Token, extra::Err<Rich<'a, char>>> {
    choice((
        text::int(10)
            .from_str()
            .unwrapped()
            .map(Token::Num),
        just('+').to(Token::Op(Op::Add)),
        just('*').to(Token::Op(Op::Mul)),
    ))
}

/// Extracts a problem from a vertical slice of the grid defined by [start_col, end_col).
fn extract_problem(lines: &[&str], start_col: usize, end_col: usize) -> Option<Problem> {
    let mut numbers = Vec::new();
    let mut op = None;

    let parser = line_content_parser();

    for line in lines {
        // Handle ragged lines (lines shorter than the current column block)
        if start_col >= line.len() {
            continue;
        }
        
        // Extract the substring for this column block
        let slice_end = std::cmp::min(end_col, line.len());
        let chunk = &line[start_col..slice_end];
        let trimmed = chunk.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Parse the trimmed chunk
        match parser.parse(trimmed).into_result() {
            Ok(Token::Num(n)) => numbers.push(n),
            Ok(Token::Op(o)) => op = Some(o),
            Err(_) => {
                // Ignore parsing errors for empty/noise lines if any, 
                // though problem guarantees clean input.
            }
        }
    }

    // A valid problem must have an operator.
    op.map(|op| Problem { numbers, op })
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Ok("0".to_string());
    }

    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let height = lines.len();

    // Identify which columns are purely whitespace separators
    let is_col_empty = |x: usize| {
        (0..height).all(|y| {
            let line = lines[y];
            if x >= line.len() {
                true // Out of bounds is treated as empty/space
            } else {
                line.as_bytes()[x] == b' '
            }
        })
    };

    let mut problems = Vec::new();

    // Group contiguous columns by whether they are empty or not
    // chunk_by is equivalent to group_by in itertools logic
    for (is_empty, cols) in (0..width).chunk_by(|&x| is_col_empty(x)).into_iter() {
        if !is_empty {
            // This is a block of content columns
            let cols_vec: Vec<usize> = cols.collect();
            let start = *cols_vec.first().unwrap();
            let end = *cols_vec.last().unwrap() + 1;

            if let Some(prob) = extract_problem(&lines, start, end) {
                problems.push(prob);
            }
        }
    }

    let total: u64 = problems.iter().map(|p| p.solve()).sum();

    Ok(total.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "123 328  51 64 
 45 64  387 23 
  6 98  215 314
*   +   *   +  ";
        assert_eq!("4277556", process(input)?);
        Ok(())
    }
}