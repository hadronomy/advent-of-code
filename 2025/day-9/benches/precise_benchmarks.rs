use aoc2025_day_9::{part1, part2};
use gungraun::{Dhat, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

// Load inputs at compile time to avoid I/O noise in the benchmark
const INPUT1: &str = include_str!("../input1.txt");
const INPUT2: &str = include_str!("../input2.txt");

#[library_benchmark]
#[bench::part1(INPUT1)]
fn bench_part1(input: &str) {
    black_box(part1::process(black_box(input)).unwrap());
}

#[library_benchmark]
#[bench::part2(INPUT2)]
fn bench_part2(input: &str) {
    black_box(part2::process(black_box(input)).unwrap());
}

library_benchmark_group!(
    name = day_9_group;
    benchmarks = bench_part1, bench_part2
);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default());
    library_benchmark_groups = day_9_group
);
