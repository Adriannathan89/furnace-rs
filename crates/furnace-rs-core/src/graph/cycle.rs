//! Canonical cycle detection for resolved provider graphs.

use std::cmp::Ordering;

use crate::SourceLocation;

use super::model::ApplicationGraph;

pub(crate) struct Cycle {
    pub(crate) names: Vec<&'static str>,
    pub(crate) location: SourceLocation,
}

/// Detects every provider cycle and returns each path in canonical name order.
pub(crate) fn detect(graph: &ApplicationGraph) -> Vec<Cycle> {
    let adjacency = adjacency(graph);
    let mut colors = vec![Color::White; graph.providers.len()];
    let mut stack = Vec::new();
    let mut cycles = Vec::new();

    for index in 0..graph.providers.len() {
        if colors[index] == Color::White {
            visit(
                index,
                graph,
                &adjacency,
                &mut colors,
                &mut stack,
                &mut cycles,
            );
        }
    }

    cycles.sort_by(|left, right| left.names.cmp(&right.names));
    cycles.dedup_by(|left, right| left.names == right.names);

    cycles
}

/// Performs depth-first coloring and records a back-edge as one dependency cycle.
fn visit(
    index: usize,
    graph: &ApplicationGraph,
    adjacency: &[Vec<usize>],
    colors: &mut [Color],
    stack: &mut Vec<usize>,
    cycles: &mut Vec<Cycle>,
) {
    colors[index] = Color::Gray;
    stack.push(index);

    for &dependency in &adjacency[index] {
        match colors[dependency] {
            Color::White => visit(dependency, graph, adjacency, colors, stack, cycles),
            Color::Gray => {
                let cycle_start = stack
                    .iter()
                    .position(|candidate| *candidate == dependency)
                    .expect("gray providers are always active");
                let open_cycle = &stack[cycle_start..];
                let canonical = canonicalize(graph, open_cycle);
                let mut names = canonical
                    .iter()
                    .map(|candidate| graph.providers[*candidate].type_name)
                    .collect::<Vec<_>>();
                names.push(names[0]);
                let location = graph.providers[canonical[0]]
                    .location
                    .expect("cycle providers are statically declared");
                cycles.push(Cycle { names, location });
            }
            Color::Black => {}
        }
    }

    stack.pop();
    colors[index] = Color::Black;
}

/// Builds every provider's dependencies once, in deterministic traversal order.
fn adjacency(graph: &ApplicationGraph) -> Vec<Vec<usize>> {
    let index = graph.index_by_type();
    let mut adjacency = vec![Vec::new(); graph.providers.len()];

    for edge in &graph.dependencies {
        if let (Some(&from), Some(&to)) = (
            index.get(&edge.provider_type_id),
            index.get(&edge.dependency_type_id),
        ) {
            adjacency[from].push(to);
        }
    }

    for dependencies in &mut adjacency {
        dependencies.sort_by(|left, right| node_order(graph, *left, *right));
    }

    adjacency
}

/// Rotates a cycle so equivalent paths share the same least-ordered starting node.
fn canonicalize(graph: &ApplicationGraph, open_cycle: &[usize]) -> Vec<usize> {
    let start = open_cycle
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| node_order(graph, **left, **right))
        .expect("cycles are non-empty")
        .0;

    open_cycle[start..]
        .iter()
        .chain(&open_cycle[..start])
        .copied()
        .collect()
}

/// Orders cycle nodes by type, origin, and optional source location.
fn node_order(graph: &ApplicationGraph, left: usize, right: usize) -> Ordering {
    let left = &graph.providers[left];
    let right = &graph.providers[right];

    left.type_name
        .cmp(right.type_name)
        .then_with(|| left.origin.cmp(&right.origin))
        .then_with(|| left.location.cmp(&right.location))
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}
