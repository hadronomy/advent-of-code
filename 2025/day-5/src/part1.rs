use chumsky::prelude::*;
use miette::*;
use std::ops::RangeInclusive;

fn parser<'a>(
) -> impl Parser<'a, &'a str, (Vec<RangeInclusive<u64>>, Vec<u64>), extra::Err<Rich<'a, char>>> {
    // Robust newline parser handling CRLF (\r\n) or LF (\n)
    let newline = just('\r').or_not().ignore_then(just('\n'));

    let range = text::int(10)
        .from_str()
        .unwrapped()
        .then_ignore(just('-'))
        .then(text::int(10).from_str().unwrapped())
        .map(|(start, end)| start..=end);

    // Block 1: Ranges
    let ranges = range.separated_by(newline).allow_trailing().collect();

    // Block 2: IDs
    let ids = text::int(10)
        .from_str()
        .unwrapped()
        .separated_by(newline)
        .allow_trailing()
        .collect();

    // Structure: Ranges -> (Trailing Sep consumed) -> Blank Line -> IDs
    ranges
        .then_ignore(newline)
        .then(ids)
        .padded()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let (ranges, ids) = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    // Count how many IDs fall into at least one fresh range
    let fresh_count = ids
        .into_iter()
        .filter(|id| ranges.iter().any(|r| r.contains(id)))
        .count();

    Ok(fresh_count.to_string())
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
        assert_eq!("3", process(input)?);
        Ok(())
    }
}
