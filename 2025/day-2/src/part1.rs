use chumsky::prelude::*;
use itertools::Itertools;
use miette::*;

/// Checks if an ID consists of a digit sequence repeated twice (e.g., 123123, 55).
fn is_invalid_id(n: u64) -> bool {
    let s = n.to_string();
    let len = s.len();

    // An ID must have even length to be two identical halves
    if len % 2 != 0 {
        return false;
    }

    let mid = len / 2;
    let (left, right) = s.split_at(mid);
    left == right
}

/// Parses a list of ranges "min-max" separated by commas.
fn parser<'a>() -> impl Parser<'a, &'a str, Vec<(u64, u64)>, extra::Err<Rich<'a, char>>> {
    let range = text::int(10)
        .from_str::<u64>()
        .unwrapped()
        .then_ignore(just('-'))
        .then(text::int(10).from_str::<u64>().unwrapped())
        .padded(); // Handles surrounding whitespace (including newlines)

    range.separated_by(just(',')).allow_trailing().collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let ranges = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let sum: u64 = ranges
        .into_iter()
        // Flatten the ranges into a single iterator of IDs
        .flat_map(|(start, end)| start..=end)
        // Check the pattern condition
        .filter(|&id| is_invalid_id(id))
        // Ensure we don't double count if the input ranges happen to overlap
        .unique()
        .sum();

    Ok(sum.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_invalid_id() {
        assert!(is_invalid_id(11));
        assert!(is_invalid_id(22));
        assert!(is_invalid_id(123123));
        assert!(is_invalid_id(446446));

        assert!(!is_invalid_id(101)); // Odd length
        assert!(!is_invalid_id(12)); // Even, no repeat
        assert!(!is_invalid_id(12123)); // Odd length
    }

    #[test]
    fn it_works() -> Result<()> {
        let input = "11-22,95-115,998-1012,1188511880-1188511890,222220-222224,
1698522-1698528,446443-446449,38593856-38593862,565653-565659,
824824821-824824827,2121212118-2121212124";
        assert_eq!("1227775554", process(input)?);
        Ok(())
    }
}
