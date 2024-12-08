use aoc2024_day_7::*;

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