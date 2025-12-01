use miette::*;
use nom::{
    character::complete::{self, line_ending, one_of},
    multi::separated_list1,
    sequence::pair,
    IResult,
};

#[derive(Debug, Clone, Copy)]
enum Instruction {
    Left(u32),
    Right(u32),
}

/// Parses a single instruction line like "L68" or "R48"
fn parse_instruction(input: &str) -> IResult<&str, Instruction> {
    let (input, (dir, amount)) = pair(one_of("LR"), complete::u32)(input)?;
    let instruction = match dir {
        'L' => Instruction::Left(amount),
        'R' => Instruction::Right(amount),
        _ => unreachable!("nom parser ensures only L or R"),
    };
    Ok((input, instruction))
}

/// Parses the full input into a vector of Instructions
fn parse(input: &str) -> IResult<&str, Vec<Instruction>> {
    separated_list1(line_ending, parse_instruction)(input)
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let (_, instructions) = parse(input).map_err(|e| miette!("Parse failed: {}", e))?;

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
