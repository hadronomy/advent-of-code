use chumsky::prelude::*;
use glam::DVec3;
use itertools::Itertools;
use miette::*;

/// A standard Disjoint Set Union (DSU) with path compression and union by size.
struct Dsu {
    parent: Vec<usize>,
    sizes: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            sizes: vec![1; n],
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let root = self.find(self.parent[i]);
            self.parent[i] = root;
            root
        }
    }

    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);

        if root_i != root_j {
            if self.sizes[root_i] < self.sizes[root_j] {
                self.parent[root_i] = root_j;
                self.sizes[root_j] += self.sizes[root_i];
            } else {
                self.parent[root_j] = root_i;
                self.sizes[root_i] += self.sizes[root_j];
            }
        }
    }

    fn get_component_sizes(&mut self) -> Vec<usize> {
        let n = self.parent.len();
        let mut components = Vec::new();
        for i in 0..n {
            if self.parent[i] == i {
                components.push(self.sizes[i]);
            }
        }
        components
    }
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<DVec3>, extra::Err<Rich<'a, char>>> {
    let coord = text::int(10).from_str::<f64>().unwrapped();

    let point = coord
        .then_ignore(just(','))
        .then(coord)
        .then_ignore(just(','))
        .then(coord)
        .map(|((x, y), z)| DVec3::new(x, y, z));

    point
        .separated_by(text::newline())
        .allow_trailing()
        .collect()
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let points = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    if points.is_empty() {
        return Ok("0".to_string());
    }

    // Generate all pairs and calculate squared Euclidean distance
    let mut edges = (0..points.len())
        .tuple_combinations()
        .map(|(i, j)| {
            let dist_sq = points[i].distance_squared(points[j]);
            (i, j, dist_sq)
        })
        .collect::<Vec<_>>();

    // Sort edges by distance (ascending).
    // f64 doesn't implement Ord, so we use partial_cmp.
    // Since inputs are integers, we won't have NaNs, so unwrap is safe.
    edges.sort_unstable_by(|(_, _, dist_a), (_, _, dist_b)| dist_a.partial_cmp(dist_b).unwrap());

    let mut dsu = Dsu::new(points.len());

    // Connect the 1000 closest pairs
    let limit = 1000.min(edges.len());

    for &(u, v, _) in edges.iter().take(limit) {
        dsu.union(u, v);
    }

    let mut sizes = dsu.get_component_sizes();

    // Get top 3 largest circuits
    sizes.sort_unstable_by(|a, b| b.cmp(a));

    let result: usize = sizes.iter().take(3).product();

    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "162,817,812
57,618,57
906,360,560
592,479,940
352,342,300
466,668,158
542,29,236
431,825,988
739,650,466
52,470,668
216,146,977
819,987,18
117,168,530
805,96,715
346,949,466
970,615,88
941,993,340
862,61,35
984,92,344
425,690,689";

        // To test strictly against the logic "10 shortest connections" from the example text:
        let points = parser().parse(input).unwrap();
        let mut edges = (0..points.len())
            .tuple_combinations()
            .map(|(i, j)| (i, j, points[i].distance_squared(points[j])))
            .collect::<Vec<_>>();

        edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        let mut dsu = Dsu::new(points.len());
        // Use 10 instead of 1000 for the unit test example check
        for &(u, v, _) in edges.iter().take(10) {
            dsu.union(u, v);
        }

        let mut sizes = dsu.get_component_sizes();
        sizes.sort_by(|a, b| b.cmp(a));
        let ans: usize = sizes.iter().take(3).product();

        assert_eq!(ans, 40);

        Ok(())
    }
}
