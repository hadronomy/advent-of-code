use chumsky::prelude::*;
use miette::*;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
struct GraphRaw {
    edges: Vec<(String, Vec<String>)>,
}

fn parser<'a>() -> impl Parser<'a, &'a str, GraphRaw, extra::Err<Rich<'a, char>>> {
    let ident = text::ident().map(ToString::to_string);

    let dests = ident.separated_by(just(' ')).collect();

    let line = ident.then_ignore(just(':').padded()).then(dests);

    line.separated_by(text::newline())
        .allow_trailing()
        .collect()
        .map(|edges| GraphRaw { edges })
}

struct Solver {
    adj: Vec<Vec<usize>>,
    start_node: usize,
    end_node: usize,
    num_nodes: usize,
}

impl Solver {
    fn new(raw: GraphRaw) -> Result<Self> {
        let mut name_to_id: HashMap<String, usize> = HashMap::new();
        let mut get_id = |name: String| {
            let len = name_to_id.len();
            *name_to_id.entry(name).or_insert(len)
        };

        // First pass: Assign IDs to all source nodes and collect edges
        // We use a temporary list because we might encounter destination nodes
        // that don't appear as source nodes (like "out" in the example).
        let mut temp_edges = Vec::new();
        for (src, dsts) in raw.edges {
            let u = get_id(src);
            for dst in dsts {
                let v = get_id(dst);
                temp_edges.push((u, v));
            }
        }

        let start_node = name_to_id
            .get("you")
            .ok_or(miette!("Node 'you' not found"))?;
        let end_node = name_to_id
            .get("out")
            .ok_or(miette!("Node 'out' not found"))?;
        let num_nodes = name_to_id.len();

        // Build adjacency list
        let mut adj = vec![Vec::new(); num_nodes];
        for (u, v) in temp_edges {
            adj[u].push(v);
        }

        Ok(Self {
            adj,
            start_node: *start_node,
            end_node: *end_node,
            num_nodes,
        })
    }

    /// Counts paths using DP on Topological Order (Kahn's Algorithm).
    /// This works because the problem guarantees data flows one way (DAG).
    fn count_paths(&self) -> u128 {
        let mut in_degree = vec![0; self.num_nodes];
        for u in 0..self.num_nodes {
            for &v in &self.adj[u] {
                in_degree[v] += 1;
            }
        }

        // Initialize queue with nodes having 0 in-degree
        let mut queue = VecDeque::new();
        for (i, &degree) in in_degree.iter().enumerate().take(self.num_nodes) {
            if degree == 0 {
                queue.push_back(i);
            }
        }

        // DP array to store number of paths from 'you' to node 'i'
        // We use u128 to prevent overflow on complex graphs
        let mut paths = vec![0u128; self.num_nodes];
        paths[self.start_node] = 1;

        // Process nodes in topological order
        while let Some(u) = queue.pop_front() {
            // Propagate path counts to neighbors
            // If paths[u] is 0 (unreachable from 'you'), it adds 0, which is correct.
            let count_u = paths[u];

            if count_u > 0 {
                for &v in &self.adj[u] {
                    paths[v] += count_u;
                }
            }

            // Standard Kahn's update
            for &v in &self.adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        paths[self.end_node]
    }
}

// -----------------------------------------------------------------------------
// Main Process
// -----------------------------------------------------------------------------

#[tracing::instrument]
pub fn process(input: &str) -> Result<String> {
    let raw_graph = parser()
        .parse(input)
        .into_result()
        .map_err(|e| miette!("Parse failed: {:?}", e))?;

    let solver = Solver::new(raw_graph)?;
    let total_paths = solver.count_paths();

    Ok(total_paths.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let input = "aaa: you hhh
you: bbb ccc
bbb: ddd eee
ccc: ddd eee fff
ddd: ggg
eee: out
fff: out
ggg: out
hhh: ccc fff iii
iii: out";
        assert_eq!("5", process(input)?);
        Ok(())
    }
}
