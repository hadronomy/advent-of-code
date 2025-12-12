use chumsky::prelude::*;
use miette::*;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
struct GraphRaw {
    edges: Vec<(String, Vec<String>)>,
}

fn parser<'a>() -> impl Parser<'a, &'a str, GraphRaw, extra::Err<Rich<'a, char>>> {
    let ident = text::ident().map(ToString::to_string);

    // Parse target list: "bbb ccc"
    let dests = ident.separated_by(just(' ')).collect();

    // Parse line: "aaa: bbb ccc"
    let line = ident.then_ignore(just(':').padded()).then(dests);

    line.separated_by(text::newline())
        .allow_trailing()
        .collect()
        .map(|edges| GraphRaw { edges })
}

struct Solver {
    adj: Vec<Vec<usize>>,
    name_to_id: HashMap<String, usize>,
    topo_order: Vec<usize>,
}

impl Solver {
    fn new(raw: GraphRaw) -> Result<Self> {
        let mut name_to_id: HashMap<String, usize> = HashMap::new();
        let mut get_id = |name: String| {
            let len = name_to_id.len();
            *name_to_id.entry(name).or_insert(len)
        };

        // Intern all node names and build edge list
        let mut temp_edges = Vec::new();
        for (src, dsts) in raw.edges {
            let u = get_id(src);
            for dst in dsts {
                let v = get_id(dst);
                temp_edges.push((u, v));
            }
        }

        let num_nodes = name_to_id.len();
        let mut adj = vec![Vec::new(); num_nodes];
        let mut in_degree = vec![0; num_nodes];

        for (u, v) in temp_edges {
            adj[u].push(v);
            in_degree[v] += 1;
        }

        // Kahn's Algorithm for Topological Sort
        let mut queue = VecDeque::new();
        for (i, &degree) in in_degree.iter().enumerate().take(num_nodes) {
            if degree == 0 {
                queue.push_back(i);
            }
        }

        let mut topo_order = Vec::with_capacity(num_nodes);
        while let Some(u) = queue.pop_front() {
            topo_order.push(u);
            for &v in &adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        // Check for cycles (though problem implies DAG)
        if topo_order.len() != num_nodes {
            return Err(miette!(
                "Graph contains a cycle; cannot process paths safely."
            ));
        }

        Ok(Self {
            adj,
            name_to_id,
            topo_order,
        })
    }

    /// Counts paths from `start_node` to `end_node` using Dynamic Programming
    /// over the pre-calculated topological order.
    fn count_paths(&self, start: &str, end: &str) -> u128 {
        let u_start = match self.name_to_id.get(start) {
            Some(&id) => id,
            None => return 0,
        };
        let u_end = match self.name_to_id.get(end) {
            Some(&id) => id,
            None => return 0,
        };

        // DP state: count of paths from `start` to node `i`
        let mut paths = vec![0u128; self.adj.len()];
        paths[u_start] = 1;

        // Iterate through nodes in topological order.
        // This ensures that when we process node u, all its incoming paths
        // (from ancestors) have been counted.
        for &u in &self.topo_order {
            // Optimization: If u is unreachable from start, skip
            if paths[u] == 0 {
                continue;
            }

            // If we've passed the end node in topological order, we technically could stop
            // if we knew u_end was visited, but iterating to the end is cheap (O(V+E)).

            for &v in &self.adj[u] {
                paths[v] += paths[u];
            }
        }

        paths[u_end]
    }
}

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let raw_graph = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let solver = Solver::new(raw_graph)?;

    // We need paths from `svr` to `out` passing through BOTH `dac` and `fft`.
    // Since it's a DAG, the order must be either:
    // 1. svr -> ... -> dac -> ... -> fft -> ... -> out
    // 2. svr -> ... -> fft -> ... -> dac -> ... -> out

    // Case 1: svr -> dac -> fft -> out
    let paths_dac_first = solver.count_paths("svr", "dac")
        * solver.count_paths("dac", "fft")
        * solver.count_paths("fft", "out");

    // Case 2: svr -> fft -> dac -> out
    let paths_fft_first = solver.count_paths("svr", "fft")
        * solver.count_paths("fft", "dac")
        * solver.count_paths("dac", "out");

    let total = paths_dac_first + paths_fft_first;

    Ok(total.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "svr: aaa bbb
aaa: fft
fft: ccc
bbb: tty
tty: ccc
ccc: ddd eee
ddd: hub
hub: fff
eee: dac
dac: fff
fff: ggg hhh
ggg: out
hhh: out";
        assert_eq!("2", process(input)?);
        Ok(())
    }
}
