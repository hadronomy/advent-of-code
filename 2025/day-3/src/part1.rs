use chumsky::prelude::*;
use miette::*;

fn max_joltage(bank: &str) -> u32 {
    let bytes = bank.as_bytes();
    let len = bytes.len();

    if len < 2 {
        return 0;
    }

    let mut max_suffix_digit = (bytes[len - 1] - b'0') as u32;
    let mut max_joltage = 0;

    for i in (0..len - 1).rev() {
        let d1 = (bytes[i] - b'0') as u32;
        let current_joltage = d1 * 10 + max_suffix_digit;

        if current_joltage > max_joltage {
            max_joltage = current_joltage;
        }

        if d1 > max_suffix_digit {
            max_suffix_digit = d1;
        }

        if max_joltage == 99 {
            return 99;
        }
    }

    max_joltage
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

    let total_joltage: u32 = banks.into_iter().map(max_joltage).sum();

    Ok(total_joltage.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_joltage() {
        assert_eq!(max_joltage("987654321111111"), 98);
        assert_eq!(max_joltage("811111111111119"), 89);
        assert_eq!(max_joltage("234234234234278"), 78);
        assert_eq!(max_joltage("818181911112111"), 92);
    }

    #[test]
    fn it_works() -> Result<()> {
        let input = "987654321111111
811111111111119
234234234234278
818181911112111";
        assert_eq!("357", process(input)?);
        Ok(())
    }
}
