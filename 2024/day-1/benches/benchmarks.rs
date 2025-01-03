use aoc2024_day_1::*;

fn main() {
    divan::main();
}

#[divan::bench]
fn part1() {
    part1::process(divan::black_box(include_str!("../input.txt",))).unwrap();
}

#[divan::bench]
fn part2() {
    part2::process(divan::black_box(include_str!("../input.txt",))).unwrap();
}

#[divan::bench]
fn part2_counter() {
    part2_counter::process(divan::black_box(include_str!("../input.txt",))).unwrap();
}
