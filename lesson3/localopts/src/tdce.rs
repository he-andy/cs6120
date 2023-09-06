use crate::utils::CFGNode;
use bril_rs::{Code, Function, Program};
use std::collections::HashSet;

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
            if let Some(def) = x.defs() {
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
        functions: prog.functions.into_iter().map(|x| global_tdce(x)).collect(),
        imports: prog.imports,
    }
}
