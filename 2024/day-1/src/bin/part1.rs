use aoc2024_day_1::part1;

fn main() {
    tracing_subscriber::fmt::init();
    let input = include_str!("../../input.txt");
    let result = part1::process(input);
    println!("Result: {}", result.expect("should have a result"));
}
