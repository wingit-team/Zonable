use wasm_bindgen::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[wasm_bindgen]
pub fn find_path(graph: &[u32], starts: &[u32], ends: &[u32]) -> Vec<u32> {
    if starts.is_empty() || ends.is_empty() {
        return Vec::new();
    }

    let start = starts[0];
    let goal = ends[0];
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();

    for edge in graph.chunks_exact(2) {
        let from = edge[0];
        let to = edge[1];
        adjacency.entry(from).or_default().push(to);
    }

    #[derive(Clone, Eq, PartialEq)]
    struct Node {
        tile: u32,
        f_score: u32,
    }

    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other.f_score.cmp(&self.f_score)
        }
    }

    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<u32, u32> = HashMap::new();
    let mut g_score: HashMap<u32, u32> = HashMap::new();
    g_score.insert(start, 0);
    open.push(Node {
        tile: start,
        f_score: heuristic(start, goal),
    });

    while let Some(Node { tile, .. }) = open.pop() {
        if tile == goal {
            return reconstruct_path(&came_from, start, goal);
        }

        let current_cost = *g_score.get(&tile).unwrap_or(&u32::MAX);
        if let Some(neighbors) = adjacency.get(&tile) {
            for next in neighbors {
                let tentative = current_cost.saturating_add(1);
                if tentative < *g_score.get(next).unwrap_or(&u32::MAX) {
                    came_from.insert(*next, tile);
                    g_score.insert(*next, tentative);
                    open.push(Node {
                        tile: *next,
                        f_score: tentative.saturating_add(heuristic(*next, goal)),
                    });
                }
            }
        }
    }

    Vec::new()
}

fn heuristic(a: u32, b: u32) -> u32 {
    let ax = a % 65_536;
    let az = a / 65_536;
    let bx = b % 65_536;
    let bz = b / 65_536;
    ax.abs_diff(bx) + az.abs_diff(bz)
}

fn reconstruct_path(came_from: &HashMap<u32, u32>, start: u32, goal: u32) -> Vec<u32> {
    let mut current = goal;
    let mut path = vec![current];

    while current != start {
        if let Some(previous) = came_from.get(&current) {
            current = *previous;
            path.push(current);
        } else {
            return Vec::new();
        }
    }

    path.reverse();
    path
}
