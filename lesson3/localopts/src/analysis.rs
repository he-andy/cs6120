use crate::cfg::{Dir, CFG};
use crate::utils::CFGNode;
use petgraph::graph::NodeIndex;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

fn union<T>(a: &HashSet<T>, b: &HashSet<T>) -> HashSet<T>
where
    T: Eq + Hash + Clone,
{
    a.union(b).cloned().collect()
}

fn intersection<T>(a: &HashSet<T>, b: &HashSet<T>) -> HashSet<T>
where
    T: Eq + Hash + Clone,
{
    a.intersection(b).cloned().collect()
}

fn live_var_transfer<T: CFGNode + Clone + Debug>(
    l: &HashSet<String>,
    n: &NodeIndex,
    cfg: &CFG<T>,
) -> HashSet<String> {
    let n = cfg.graph.node_weight(*n).unwrap();
    n.uses().union(&(l - &n.defs())).cloned().collect()
}

pub fn live_variable_analysis<T: CFGNode + Clone + std::fmt::Debug>(
    cfg: &CFG<T>,
) -> (Vec<HashSet<String>>, Vec<HashSet<String>>) {
    cfg.work_list(
        union,
        live_var_transfer,
        HashSet::<String>::new(),
        HashSet::<String>::new(),
        Dir::Backward,
        true,
    )
}
