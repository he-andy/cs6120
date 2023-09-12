use crate::utils::{basic_blocks, CFGNode};
use bril_rs::{Code, Function, Program};
use std::collections::{HashMap, HashSet};

pub fn global_pass(prog: Program) -> Program {
    Program {
        functions: prog.functions.into_iter().map(|x| global_tdce(x)).collect(),
        imports: prog.imports,
    }
}

fn global_tdce(func: Function) -> Function {
    let mut code = func.instrs;
    let mut changed = true;
    while changed {
        (changed, code) = global_tdce_single_pass(code);
    }

    Function {
        instrs: code,
        ..func
    }
}

fn global_tdce_single_pass(code: Vec<Code>) -> (bool, Vec<Code>) {
    let used = code
        .iter()
        .map(|x| x.uses())
        .flatten()
        .collect::<HashSet<_>>();

    let to_delete = code
        .iter()
        .enumerate()
        .map(|(i, x)| {
            if let Some(def) = x.defs().iter().next().cloned() {
                if !used.contains(&def) {
                    Some(i)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .filter_map(|x| x)
        .collect::<HashSet<_>>();

    let changed = !to_delete.is_empty();

    (
        changed,
        code.into_iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if to_delete.contains(&i) {
                    None
                } else {
                    Some(x)
                }
            })
            .collect(),
    )
}

pub fn local_pass(prog: Program) -> Program {
    Program {
        functions: prog.functions.into_iter().map(|x| local_tdce(x)).collect(),
        imports: prog.imports,
    }
}

fn local_tdce(func: Function) -> Function {
    let local_dce_pass = basic_blocks(func.instrs)
        .into_iter()
        .map(|block| {
            let mut changed = true;
            let mut block = block.instructions;
            while changed {
                (changed, block) = local_tdce_single_pass(block);
            }
            block
        })
        .flatten()
        .collect();

    Function {
        instrs: local_dce_pass,
        ..func
    }
}

fn local_tdce_single_pass(code: Vec<Code>) -> (bool, Vec<Code>) {
    let mut last_def = HashMap::new();
    let mut to_delete = HashSet::new();
    for (line, ins) in code.iter().enumerate() {
        //check for uses
        for r#use in ins.uses() {
            last_def.remove(&r#use);
        }
        //check for defs
        if let Some(def) = ins.defs().iter().next().cloned() {
            if let Some(idx) = last_def.insert(def, line) {
                to_delete.insert(idx);
            }
        }
    }

    let changed = !to_delete.is_empty();

    (
        changed,
        code.into_iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if to_delete.contains(&i) {
                    None
                } else {
                    Some(x)
                }
            })
            .collect(),
    )
}
