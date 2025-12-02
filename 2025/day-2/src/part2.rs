use chumsky::prelude::*;
use itertools::Itertools;
use miette::*;

/// Checks if an ID consists of a digit sequence repeated at least twice.
/// # Examples:
/// ```no_run
/// 11 (1 repeated 2 times) -> Valid
/// 123123 (123 repeated 2 times) -> Valid
/// 121212 (12 repeated 3 times) -> Valid
/// ```
#[allow(dead_code)]
fn is_invalid_id(n: u64) -> bool {
    let s = n.to_string();
    let len = s.len();
    let bytes = s.as_bytes();

    // We try all possible lengths for the repeating pattern,
    // from 1 up to half the length of the string.
    for pattern_len in 1..=len / 2 {
        // The total length must be divisible by the pattern length to be a perfect repetition
        if len % pattern_len == 0 {
            let pattern = &bytes[0..pattern_len];

            // Check if every chunk of size pattern_len matches the pattern
            let is_match = bytes.chunks(pattern_len).all(|chunk| chunk == pattern);

            if is_match {
                return true;
            }
        }
    }

    false
}

/// Checks if an ID consists of a digit sequence repeated at least twice.
/// # Examples:
/// ```no_run
/// 11 (1 repeated 2 times) -> Valid
/// 123123 (123 repeated 2 times) -> Valid
/// 121212 (12 repeated 3 times) -> Valid
/// ```
/// # Note
///
/// This optimized version avoids string manipulation.
/// Hyperfine bechmark results
///
/// ```no_run
/// Time (mean ± σ):     155.9 ms ±   2.9 ms    [User: 144.0 ms, System: 29.4 ms]
/// Range (min … max):   152.1 ms … 162.1 ms    18 runs
/// ```
/// vs the [original][is_invalid_id] implementation:
/// ```no_run
/// Time (mean ± σ):     214.0 ms ±   2.4 ms    [User: 211.8 ms, System: 26.1 ms]
/// Range (min … max):   211.8 ms … 220.4 ms    14 runs
/// ```
pub fn is_invalid_id_optimized(n: u64) -> bool {
    // Single digits cannot repeat twice
    if n < 10 {
        return false;
    }

    // 1. Get total number of digits without string conversion (intrinsic CPU instruction)
    let num_digits = n.ilog10() + 1;

    // 2. Iterate possible pattern lengths
    // We only need to check lengths up to half the total digits
    for pattern_len in 1..=(num_digits / 2) {
        // A pattern can only be valid if it divides the total length evenly
        if num_digits % pattern_len != 0 {
            continue;
        }

        // 3. Construct the "repetition mask"
        // If num_digits = 6 and pattern_len = 3:
        // shift_block = 10^3 = 1000
        // Mask construction loop:
        //   Iter 1: 1
        //   Iter 2: 1 * 1000 + 1 = 1001
        let shift_block = 10_u64.pow(pattern_len);
        let mut mask = 1;
        let repeats = num_digits / pattern_len;

        // Build the mask: 1 -> 101 -> 10101...
        for _ in 1..repeats {
            mask = mask * shift_block + 1;
        }

        // 4. Verification
        // If n is perfectly divisible by the mask, it is a repetition.
        if n % mask == 0 {
            return true;
        }
    }

    false
}

/// Parses a list of ranges "min-max" separated by commas.
fn parser<'a>() -> impl Parser<'a, &'a str, Vec<(u64, u64)>, extra::Err<Rich<'a, char>>> {
    let range = text::int(10)
        .from_str::<u64>()
        .unwrapped()
        .then_ignore(just('-'))
        .then(text::int(10).from_str::<u64>().unwrapped())
        .padded(); // Handles whitespace around tokens

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
        // Flatten ranges into a single stream of IDs
        .flat_map(|(start, end)| start..=end)
        // Check the repeating pattern condition
        .filter(|&id| is_invalid_id(id))
        // Ensure unique IDs if ranges overlap
        .unique()
        .sum();

    Ok(sum.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_invalid_id_part_2() {
        // Part 1 cases
        assert!(is_invalid_id(11));
        assert!(is_invalid_id(22));
        assert!(is_invalid_id(123123));

        // Part 2 new cases
        assert!(is_invalid_id(12341234)); // 1234 x 2
        assert!(is_invalid_id(123123123)); // 123 x 3
        assert!(is_invalid_id(1212121212)); // 12 x 5
        assert!(is_invalid_id(1111111)); // 1 x 7
        assert!(is_invalid_id(999)); // 9 x 3
        assert!(is_invalid_id(1010)); // 10 x 2

        // Negative cases
        assert!(!is_invalid_id(101));
        assert!(!is_invalid_id(12345));
        assert!(!is_invalid_id(12123));
    }

    #[test]
    fn test_is_invalid_id_optimized_part_2() {
        // Part 1 cases
        assert!(is_invalid_id_optimized(11));
        assert!(is_invalid_id_optimized(22));
        assert!(is_invalid_id_optimized(123123));
        // Part 2 new cases
        assert!(is_invalid_id_optimized(12341234)); // 1234 x 2
        assert!(is_invalid_id_optimized(123123123)); // 123 x 3
        assert!(is_invalid_id_optimized(1212121212)); // 12 x 5
        assert!(is_invalid_id_optimized(1111111)); // 1 x 7
        assert!(is_invalid_id_optimized(999)); // 9 x 3
        assert!(is_invalid_id_optimized(1010)); // 10 x 2

        // Negative cases
        assert!(!is_invalid_id_optimized(101));
        assert!(!is_invalid_id_optimized(12345));
        assert!(!is_invalid_id_optimized(12123));
    }

    #[test]
    fn it_works() -> Result<()> {
        let input = "11-22,95-115,998-1012,1188511880-1188511890,222220-222224,
1698522-1698528,446443-446449,38593856-38593862,565653-565659,
824824821-824824827,2121212118-2121212124";
        assert_eq!("4174379265", process(input)?);
        Ok(())
    }
}
