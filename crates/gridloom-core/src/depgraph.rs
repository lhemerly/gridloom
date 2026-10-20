use crate::error::CellError;
use crate::value::{CellCoord, CellRange};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DepNode {
    Cell(CellCoord),
    Range(CellRange),
    Table(String),
    Source(String),
    Pivot(String),
    Chart(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Mapping from node -> set of nodes that depend on it (downstream dependents)
    pub dependents: HashMap<DepNode, HashSet<DepNode>>,
    /// Mapping from node -> set of nodes it depends on (upstream dependencies)
    pub dependencies: HashMap<DepNode, HashSet<DepNode>>,
    pub generation: u64,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependents: HashMap::new(),
            dependencies: HashMap::new(),
            generation: 1,
        }
    }

    /// Clears existing upstream dependencies for a node before re-parsing
    pub fn clear_dependencies(&mut self, node: &DepNode) {
        if let Some(upstream) = self.dependencies.remove(node) {
            for dep in upstream {
                if let Some(set) = self.dependents.get_mut(&dep) {
                    set.remove(node);
                }
            }
        }
    }

    /// Add dependency: `node` depends on `depends_on`.
    /// Checks for cycle and returns `Err(CellError::Cycle)` if detected.
    pub fn add_dependency(&mut self, node: DepNode, depends_on: DepNode) -> Result<(), CellError> {
        if node == depends_on {
            return Err(CellError::Cycle);
        }

        // Check if depends_on already depends on node (would create cycle)
        if self.path_exists(&node, &depends_on) {
            return Err(CellError::Cycle);
        }

        self.dependencies
            .entry(node.clone())
            .or_default()
            .insert(depends_on.clone());
        self.dependents
            .entry(depends_on)
            .or_default()
            .insert(node);

        Ok(())
    }

    /// BFS check if a path exists from `from` to `to` in the dependents graph
    pub fn path_exists(&self, from: &DepNode, to: &DepNode) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());
        visited.insert(from.clone());

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&current) {
                for next in deps {
                    if next == to {
                        return true;
                    }
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        false
    }

    /// Finds all downstream dependents that need recalculation when `dirty_roots` change
    pub fn get_recalculation_order(&self, dirty_roots: &[DepNode]) -> Result<Vec<DepNode>, CellError> {
        // Collect all affected nodes
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for root in dirty_roots {
            affected.insert(root.clone());
            queue.push_back(root.clone());
        }

        while let Some(node) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&node) {
                for next in deps {
                    if affected.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        // Kahn's algorithm for topological sorting of affected nodes
        let mut in_degree: HashMap<DepNode, usize> = HashMap::new();
        for node in &affected {
            in_degree.insert(node.clone(), 0);
        }

        for node in &affected {
            if let Some(deps) = self.dependents.get(node) {
                for downstream in deps {
                    if affected.contains(downstream) {
                        *in_degree.entry(downstream.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut ready: VecDeque<DepNode> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut order = Vec::new();
        while let Some(node) = ready.pop_front() {
            order.push(node.clone());

            if let Some(deps) = self.dependents.get(&node) {
                for downstream in deps {
                    if affected.contains(downstream) {
                        if let Some(deg) = in_degree.get_mut(downstream) {
                            *deg -= 1;
                            if *deg == 0 {
                                ready.push_back(downstream.clone());
                            }
                        }
                    }
                }
            }
        }

        if order.len() < affected.len() {
            return Err(CellError::Cycle);
        }

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        let a = DepNode::Cell(CellCoord::new(0, 0)); // A1
        let b = DepNode::Cell(CellCoord::new(1, 0)); // A2
        let c = DepNode::Cell(CellCoord::new(2, 0)); // A3

        // A2 depends on A1
        graph.add_dependency(b.clone(), a.clone()).unwrap();
        // A3 depends on A2
        graph.add_dependency(c.clone(), b.clone()).unwrap();

        let order = graph.get_recalculation_order(&[a.clone()]).unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        let a = DepNode::Cell(CellCoord::new(0, 0));
        let b = DepNode::Cell(CellCoord::new(1, 0));

        graph.add_dependency(b.clone(), a.clone()).unwrap();
        // A depends on B -> cycle!
        let err = graph.add_dependency(a.clone(), b.clone());
        assert_eq!(err, Err(CellError::Cycle));
    }
}
