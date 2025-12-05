use chumsky::prelude::*;
use miette::*;
use std::ops::RangeInclusive;

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<RangeInclusive<u64>>, extra::Err<Rich<'a, char>>> {
    // Robust newline parser handling CRLF (\r\n) or LF (\n)
    let newline = just('\r').or_not().ignore_then(just('\n'));

    let range = text::int(10)
        .from_str()
        .unwrapped()
        .then_ignore(just('-'))
        .then(text::int(10).from_str().unwrapped())
        .map(|(start, end)| start..=end);

    // Block 1: Ranges
    let ranges = range
        .separated_by(newline)
        .allow_trailing()
        .collect();

    // Block 2: IDs (we interpret and discard these to consume the full input properly)
    let ids = text::int(10)
        .from_str::<u64>()
        .unwrapped()
        .separated_by(newline)
        .allow_trailing()
        .collect::<Vec<_>>();

    ranges.then_ignore(newline).then_ignore(ids).padded()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let mut ranges = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    // Sort ranges by start position to enable linear merge scan
    ranges.sort_by_key(|r| *r.start());

    let mut total_fresh_count: u64 = 0;

    // Iterate through sorted ranges and merge them
    if let Some(first) = ranges.first() {
        let mut current_start = *first.start();
        let mut current_end = *first.end();

        for r in ranges.iter().skip(1) {
            let next_start = *r.start();
            let next_end = *r.end();

            // Check if ranges overlap or are adjacent (contiguous integers)
            // e.g., 3-5 and 6-8 should merge into 3-8.
            if next_start <= current_end + 1 {
                // Merge: extend the current end if the next one goes further
                if next_end > current_end {
                    current_end = next_end;
                }
            } else {
                // Gap detected: The current merged range is complete
                total_fresh_count += current_end - current_start + 1;

                // Start tracking the new range
                current_start = next_start;
                current_end = next_end;
            }
        }
        // Don't forget to add the last range
        total_fresh_count += current_end - current_start + 1;
    }

    Ok(total_fresh_count.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "3-5
10-14
16-20
12-18

1
5
8
11
17
32";
        assert_eq!("14", process(input)?);
        Ok(())
    }
}
