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

/// Parses a single line: "L68" or "R48"
fn parse_instruction(input: &str) -> IResult<&str, Instruction> {
    let (input, (dir, amount)) = pair(one_of("LR"), complete::u32)(input)?;

    let instruction = match dir {
        'L' => Instruction::Left(amount),
        'R' => Instruction::Right(amount),
        _ => unreachable!("nom one_of ensures only L or R"),
    };

    Ok((input, instruction))
}

/// Parses the entire input into a list of Instructions
fn parse(input: &str) -> IResult<&str, Vec<Instruction>> {
    separated_list1(line_ending, parse_instruction)(input)
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let (_, instructions) = parse(input).map_err(|e| miette!("Parse failed: {}", e))?;

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
