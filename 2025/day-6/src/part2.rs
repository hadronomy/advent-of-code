use miette::Result;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Mul,
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let lines: Vec<&[u8]> = input.lines().map(|l| l.as_bytes()).collect();
    if lines.is_empty() {
        return Ok("0".to_string());
    }

    // Grid Dimensions
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);

    // A column is a separator if it contains ONLY spaces.
    let mut is_separator = vec![true; width];

    for line in &lines {
        for (x, &byte) in line.iter().enumerate() {
            if byte != b' ' {
                // Found a non-space, so this column is part of a problem block
                is_separator[x] = false;
            }
        }
    }

    // Group contiguous non-separator columns into ranges.
    let mut blocks = Vec::with_capacity(width / 4);
    let mut start = None;
    for (x, &sep) in is_separator.iter().enumerate() {
        if !sep {
            if start.is_none() {
                start = Some(x);
            }
        } else if let Some(s) = start {
            blocks.push(s..x);
            start = None;
        }
    }
    if let Some(s) = start {
        blocks.push(s..width);
    }

    // Solve Blocks in Parallel
    let grand_total: u64 = blocks
        .into_par_iter()
        .map(|range| {
            let mut numbers = Vec::with_capacity(range.len());
            let mut op = Op::Add;

            // Iterate over each column in the identified block
            for x in range {
                let mut num = 0u64;
                let mut has_digits = false;

                // Vertical Scan: Top-to-Bottom (Most Significant Digit to Least)
                for line in &lines {
                    if x >= line.len() {
                        continue;
                    }
                    let b = line[x];

                    if b.is_ascii_digit() {
                        num = num * 10 + (b - b'0') as u64;
                        has_digits = true;
                    } else if b == b'+' {
                        op = Op::Add;
                    } else if b == b'*' {
                        op = Op::Mul;
                    }
                }

                if has_digits {
                    numbers.push(num);
                }
            }

            // Reduce based on the operator found in the block
            match op {
                Op::Add => numbers.iter().sum::<u64>(),
                Op::Mul => numbers.iter().product::<u64>(),
            }
        })
        .sum();

    Ok(grand_total.to_string())
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
        assert_eq!("3263827", process(input)?);
        Ok(())
    }
}
