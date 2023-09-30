use std::collections::HashMap;

use crate::{cfg::CFG, dominator::dominator_analyis, utils::BasicBlock};
use petgraph::{algo::dominators::Dominators, graph::NodeIndex};

pub fn to_ssa(cfg: &CFG<BasicBlock>) -> CFG<BasicBlock> {
    let mut ssa = cfg.clone();
    let (doms, dom_frontier) = dominator_analyis(&ssa);

    let mut defsites = HashMap::new();
    let mut defs = vec![];
    for node in ssa.graph.node_indices() {
        let block = ssa.graph.node_weight(node).unwrap();
        for def in &block.defs {
            defsites.entry(def.clone()).or_insert(vec![]).push(node);
            defs.push(def.clone());
        }
    }

    // insert phi nodes
    for def in defs.iter() {
        let mut worklist = defsites.get(def).unwrap().clone();
        while !worklist.is_empty() {
            let defsite = worklist.pop().unwrap();
            let df = dom_frontier.get(defsite.index()).unwrap();
            for node in df {
                let block = ssa.graph.node_weight_mut(*node).unwrap();
                if !block.phi.contains_key(def) {
                    block.phi.insert(def.clone(), (def.clone(), vec![]));
                    worklist.push(*node);
                }
            }
        }
    }
    let entry = ssa.start();

    let mut stack = ssa
        .args
        .iter()
        .map(|x| (x.name.clone(), (0, vec![x.name.clone()])))
        .collect::<HashMap<_, _>>();
    rename_vars(&mut ssa, entry, &mut stack, &doms);
    return ssa;
}

fn get_last_def(stack: &mut HashMap<String, (usize, Vec<String>)>, var: &String) -> String {
    match stack.get(var) {
        Some(x) => x.1.last().unwrap().clone(),
        None => {
            stack.entry(var.clone()).or_insert((0, vec![var.clone()]));
            var.clone() //should only happen on args
        }
    }
}

fn new_def(stack: &mut HashMap<String, (usize, Vec<String>)>, var: &String) -> String {
    stack.entry(var.clone()).or_insert((0, vec![]));

    let (count, stack) = stack.get_mut(var).unwrap();
    let new_var = format!("_phi_{}_{}", var, count);
    stack.push(new_var.clone());
    *count += 1;
    new_var
}

fn pop_defs(stack: &mut HashMap<String, (usize, Vec<String>)>, new_def_ct: &HashMap<String, i32>) {
    for (var, ct) in new_def_ct {
        let (_, stack) = stack.get_mut(var).unwrap();
        for _ in 0..*ct {
            stack.pop();
        }
    }
}

fn rename_vars(
    cfg: &mut CFG<BasicBlock>,
    block_idx: NodeIndex,
    stack: &mut HashMap<String, (usize, Vec<String>)>,
    dominator: &Dominators<NodeIndex>,
) {
    let block = cfg.graph.node_weight_mut(block_idx).unwrap();
    let mut new_def_ct = HashMap::new();

    for (canonical, (phi_dest, _phi_src)) in block.phi.iter_mut() {
        *new_def_ct.entry(canonical.clone()).or_insert(0) += 1;
        let new_var = new_def(stack, canonical);
        *phi_dest = new_var;
    }

    for instr in block.instructions.iter_mut() {
        match instr {
            bril_rs::Code::Label { .. } => (),
            bril_rs::Code::Instruction(ins) => match ins {
                bril_rs::Instruction::Constant { dest, .. } => {
                    *new_def_ct.entry(dest.clone()).or_insert(0) += 1;
                    let new_var = new_def(stack, dest);
                    *dest = new_var.clone();
                }
                bril_rs::Instruction::Value { args, dest, .. } => {
                    *new_def_ct.entry(dest.clone()).or_insert(0) += 1;
                    for arg in args {
                        let new_var = get_last_def(stack, arg);
                        *arg = new_var;
                    }
                    let new_var = new_def(stack, dest);
                    *dest = new_var;
                }
                bril_rs::Instruction::Effect { args, .. } => {
                    for arg in args {
                        let new_var = get_last_def(stack, arg);
                        *arg = new_var;
                    }
                }
            },
        }
    }
    let label = block.label.clone().unwrap();

    //rename phi
    let mut successors = cfg.graph.neighbors(block_idx).detach();
    while let Some(succ_idx) = successors.next_node(&cfg.graph) {
        let succ = cfg.graph.node_weight_mut(succ_idx).unwrap();
        for (canonical, (_, phi_source)) in succ.phi.iter_mut() {
            let var_stack = stack.get(canonical);
            if var_stack.is_some_and(|x| !x.1.is_empty()) {
                phi_source.push((get_last_def(stack, canonical), label.clone()));
            }
        }
    }
    for idom in dominator.immediately_dominated_by(block_idx) {
        if idom != block_idx {
            rename_vars(cfg, idom, stack, dominator);
        }
    }

    pop_defs(stack, &new_def_ct);
}
