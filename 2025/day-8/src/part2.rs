use chumsky::prelude::*;
use glam::DVec3;
use itertools::Itertools;
use miette::*;

/// Disjoint Set Union (DSU) tracking the number of active components.
struct Dsu {
    parent: Vec<usize>,
    /// Tracks how many disjoint sets currently exist.
    num_components: usize,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            num_components: n,
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

    /// Unifies sets. Returns `true` if a merge actually occurred (sets were disjoint).
    fn union(&mut self, i: usize, j: usize) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);

        if root_i != root_j {
            // Arbitrary linking since we don't need rank/size optimizations for just this logic,
            // but path compression in find() handles the complexity.
            self.parent[root_i] = root_j;
            self.num_components -= 1;
            true
        } else {
            false
        }
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

    if points.len() < 2 {
        return Ok("0".to_string());
    }

    // Generate all edges: (u_index, v_index, distance_squared)
    let mut edges = (0..points.len())
        .tuple_combinations()
        .map(|(i, j)| {
            let dist_sq = points[i].distance_squared(points[j]);
            (i, j, dist_sq)
        })
        .collect::<Vec<_>>();

    // Sort edges ascending by distance
    edges.sort_unstable_by(|(_, _, dist_a), (_, _, dist_b)| dist_a.partial_cmp(dist_b).unwrap());

    let mut dsu = Dsu::new(points.len());

    // Iterate through edges until the graph is fully connected
    for (u, v, _) in edges {
        // Try to merge the two components
        if dsu.union(u, v) {
            // If this merge reduced the component count to 1,
            // the graph is now fully connected.
            if dsu.num_components == 1 {
                let x1 = points[u].x as i64;
                let x2 = points[v].x as i64;
                let result = x1 * x2;
                return Ok(result.to_string());
            }
        }
    }

    Err(miette!("Graph could not be fully connected"))
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

        assert_eq!("25272", process(input)?);
        Ok(())
    }
}
