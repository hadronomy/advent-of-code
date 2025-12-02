use miette::*;
use chumsky::prelude::*;

#[derive(Debug, Clone, Copy)]
enum Instruction {
    Left(u32),
    Right(u32),
}

/// Defines the parser using Chumsky combinators.
///
/// We specify the error type `extra::Err<Rich<'a, char>>` to get detailed diagnostics,
/// although we just flatten them for the result here.
fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Instruction>, extra::Err<Rich<'a, char>>> {
    let instruction = one_of("LR")
        .then(text::int(10).from_str::<u32>().unwrapped())
        .map(|(dir, amount)| match dir {
            'L' => Instruction::Left(amount),
            'R' => Instruction::Right(amount),
            _ => unreachable!("one_of ensures only L or R are parsed"),
        });

    instruction
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let instructions = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed with errors: {:?}", e))?;

    let start_pos: i64 = 50;
    let modulus: i64 = 100;

    // We use fold to maintain the current position and accumulate the total count of '0' hits.
    let (_, total_hits) = instructions
        .iter()
        .fold((start_pos, 0), |(pos, count), instruction| {
            match instruction {
                Instruction::Left(amount) => {
                    let amount = *amount as i64;

                    // Moving Left means subtracting.
                    // We cover the interval of integers [pos - amount, pos - 1].
                    // The number of multiples of 100 in an interval [A, B] is:
                    // floor(B / 100) - floor((A - 1) / 100)

                    let upper = pos - 1;
                    let lower_minus_1 = pos - amount - 1;

                    let hits = upper.div_euclid(modulus) - lower_minus_1.div_euclid(modulus);
                    let new_pos = (pos - amount).rem_euclid(modulus);

                    (new_pos, count + hits)
                }
                Instruction::Right(amount) => {
                    let amount = *amount as i64;

                    // Moving Right means adding.
                    // We cover the interval (pos, pos + amount].
                    // Since 'pos' is always normalized (0 <= pos < 100),
                    // the formula simplifies to just integer division.

                    let hits = (pos + amount) / modulus;
                    let new_pos = (pos + amount) % modulus;

                    (new_pos, count + hits)
                }
            }
        });

    Ok(total_hits.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "L68
L30
R48
L5
R60
L55
L1
L99
R14
L82";
        assert_eq!("6", process(input)?);
        Ok(())
    }
}
