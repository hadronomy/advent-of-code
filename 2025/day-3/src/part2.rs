use chumsky::prelude::*;
use miette::*;

/// Finds the largest integer that can be formed by keeping exactly `k` digits
/// from the input string `s` while preserving their relative order.
fn find_max_subsequence(s: &str, k: usize) -> u64 {
    let digits = s.as_bytes();
    let n = digits.len();

    if n < k {
        return 0;
    }

    let to_remove = n - k;
    let mut stack: Vec<u8> = Vec::with_capacity(k);
    let mut removed_count = 0;

    for &digit in digits {
        while removed_count < to_remove && !stack.is_empty() && *stack.last().unwrap() < digit {
            stack.pop();
            removed_count += 1;
        }
        stack.push(digit);
    }

    stack.truncate(k);

    // '0' in ASCII is 48. So we subtract b'0' to get the integer value 0-9.
    stack.into_iter().fold(0u64, |acc, digit_byte| {
        acc * 10 + (digit_byte - b'0') as u64
    })
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, extra::Err<Rich<'a, char>>> {
    text::digits(10)
        .to_slice()
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let banks = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let k = 12;

    let total_joltage: u64 = banks
        .into_iter()
        .map(|bank| find_max_subsequence(bank, k))
        .sum();

    Ok(total_joltage.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_max_subsequence() {
        assert_eq!(find_max_subsequence("987654321111111", 12), 987654321111);
        assert_eq!(find_max_subsequence("811111111111119", 12), 811111111119);
        assert_eq!(find_max_subsequence("234234234234278", 12), 434234234278);
        assert_eq!(find_max_subsequence("818181911112111", 12), 888911112111);
    }

    #[test]
    fn it_works() -> Result<()> {
        let input = "987654321111111
811111111111119
234234234234278
818181911112111";
        assert_eq!("3121910778619", process(input)?);
        Ok(())
    }
}
