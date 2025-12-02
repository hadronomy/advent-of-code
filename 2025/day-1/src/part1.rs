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

    let start_position = 50;
    let modulus = 100;

    let zero_hits = instructions
        .iter()
        // scan maintains the state (current dial position) through the iterator
        .scan(start_position, |position, instruction| {
            match instruction {
                // For Left: (current - amount) % 100
                // We use rem_euclid to handle negative wrapping correctly (e.g., -10 % 100 should be 90)
                Instruction::Left(amount) => {
                    *position = (*position as i32 - *amount as i32).rem_euclid(modulus) as u32;
                }
                // For Right: (current + amount) % 100
                Instruction::Right(amount) => {
                    *position = (*position + *amount) % modulus as u32;
                }
            }
            Some(*position)
        })
        .filter(|&pos| pos == 0)
        .count();

    Ok(zero_hits.to_string())
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
        assert_eq!("3", process(input)?);
        Ok(())
    }
}
