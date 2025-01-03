use aoc2024_day_3::*;

fn main() {
    divan::main();
}

#[divan::bench]
fn part1() {
    part1::process(divan::black_box(include_str!("../input1.txt",))).unwrap();
}

#[divan::bench]
fn part2() {
    part2::process(divan::black_box(include_str!("../input2.txt",))).unwrap();
}

#[divan::bench]
fn part2_pest() {
    part2_pest::process(divan::black_box(include_str!("../input2.txt",))).unwrap();
}
