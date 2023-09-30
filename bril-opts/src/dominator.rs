use crate::dominator;
use crate::utils::{BasicBlock, CFGNode};
use petgraph::algo::dominators::Dominators;
use petgraph::{graph::NodeIndex, Direction};
use std::collections::HashSet;
use std::vec;

use crate::cfg::CFG;

/// Build dominance frontier
fn compute_dominance_frontier<T: Clone + std::fmt::Debug + std::fmt::Display + CFGNode>(
    dominators: &Dominators<NodeIndex>,
    cfg: &CFG<T>,
) -> Vec<HashSet<NodeIndex>> {
    //computes all nodes dominated by node
    fn dom_by(
        node: NodeIndex,
        dominators: &Dominators<NodeIndex>,
        memo: &mut Vec<Option<HashSet<NodeIndex>>>,
    ) {
        if memo[node.index()].is_some() {
            return;
        }

        let mut dom = HashSet::from([node]);
        for idom in dominators.immediately_dominated_by(node) {
            if idom.index() == node.index() {
                continue;
            }
            dom_by(idom, dominators, memo);
            dom.extend(memo[idom.index()].as_ref().unwrap());
        }
        memo[node.index()] = Some(dom);
    }

    //helper function to compute the dominance frontier of each node
    fn dom_frontier_recur<T: Clone + std::fmt::Debug + CFGNode>(
        node: NodeIndex,
        dominators: &Dominators<NodeIndex>,
        dom_by: &Vec<HashSet<NodeIndex>>,
        cfg: &CFG<T>,
        memo: &mut Vec<Option<HashSet<NodeIndex>>>,
    ) {
        if memo[node.index()].is_some() {
            return;
        }

        let mut DF: HashSet<NodeIndex> = cfg
            .graph
            .neighbors_directed(node, Direction::Outgoing)
            .collect::<HashSet<_>>();

        for idom in dominators.immediately_dominated_by(node) {
            if idom.index() != node.index() {
                dom_frontier_recur(idom, dominators, dom_by, cfg, memo);
                DF.extend(memo[idom.index()].as_ref().unwrap());
            }
        }

        let mut dom_by_node = (&dom_by[node.index()]).clone();
        dom_by_node.remove(&node);

        memo[node.index()] = Some(
            DF.difference(&dom_by_node)
                .map(|x| *x)
                .collect::<HashSet<_>>(),
        );
    }

    let mut memo = vec![None; cfg.graph.node_count()];
    for node in cfg.graph.node_indices() {
        dom_by(node, dominators, &mut memo);
    }
    //dom_by(cfg.start(), dominators, &mut memo);
    let dom_by = memo.into_iter().map(|x| x.unwrap()).collect::<Vec<_>>();
    let mut memo = vec![None; cfg.graph.node_count()];
    for node in cfg.graph.node_indices() {
        dom_frontier_recur(node, dominators, &dom_by, cfg, &mut memo);
    }

    memo.into_iter().map(|x| x.unwrap()).collect::<Vec<_>>()
}

pub fn dominator_analyis<T: Clone + std::fmt::Debug + std::fmt::Display + CFGNode>(
    cfg: &CFG<T>,
) -> (Dominators<NodeIndex>, Vec<HashSet<NodeIndex>>) {
    let dominators = petgraph::algo::dominators::simple_fast(&cfg.graph, cfg.start());
    let dom_frontier = compute_dominance_frontier(&dominators, cfg);
    (dominators, dom_frontier)
}

pub fn dom_tree(
    dominators: &Dominators<NodeIndex>,
    cfg: &CFG<BasicBlock>,
) -> petgraph::Graph<String, ()> {
    let mut dom_tree = petgraph::Graph::<String, ()>::new();
    for node in cfg.graph.node_indices() {
        dom_tree.add_node(
            cfg.graph
                .node_weight(node)
                .unwrap()
                .label
                .clone()
                .unwrap_or("".to_string()),
        );
    }
    for node in cfg.graph.node_indices() {
        for idom in dominators.immediately_dominated_by(node) {
            if idom != node {
                dom_tree.add_edge(
                    NodeIndex::from(node.index() as u32),
                    NodeIndex::from(idom.index() as u32),
                    (),
                );
            }
        }
    }
    dom_tree
}
