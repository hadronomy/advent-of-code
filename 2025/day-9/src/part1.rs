use chumsky::prelude::*;
use itertools::Itertools;
use miette::*;

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let coord = text::int::<&str, extra::Err<Rich<char>>>(10)
        .from_str::<i64>()
        .unwrapped();

    let parser = coord
        .then_ignore(just(','))
        .then(coord)
        .separated_by(text::newline())
        .allow_trailing()
        .collect::<Vec<(i64, i64)>>();

    let points = parser
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    // Iterate over all unique pairs of points to find the max area.
    // Area of inclusive rectangle defined by opposite corners (x1,y1) and (x2,y2)
    // is (|x1 - x2| + 1) * (|y1 - y2| + 1).
    let max_area = points
        .iter()
        .tuple_combinations()
        .map(|(p1, p2)| {
            let w = (p1.0 - p2.0).unsigned_abs() + 1;
            let h = (p1.1 - p2.1).unsigned_abs() + 1;
            w * h
        })
        .max()
        .unwrap_or(0);

    Ok(max_area.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "7,1
11,1
11,7
9,7
9,5
2,5
2,3
7,3";
        assert_eq!("50", process(input)?);
        Ok(())
    }
}
