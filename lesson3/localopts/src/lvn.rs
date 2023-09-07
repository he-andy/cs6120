use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use bril_rs::{Code, ConstOps, Function, Instruction, Literal, Program, Type, ValueOps};

use crate::utils::{basic_blocks, CFGNode};

pub fn lvn(prog: Program) -> Program {
    Program {
        functions: prog.functions.into_iter().map(|x| lvn_pass(x)).collect(),
        ..prog
    }
}

fn lvn_pass(func: Function) -> Function {
    Function {
        instrs: basic_blocks(func.instrs)
            .into_iter()
            .map(|x| lvn_bb_pass(x))
            .flatten()
            .collect(),
        ..func
    }
}

fn lvn_bb_pass(code: Vec<Code>) -> Vec<Code> {
    let mut new_block = vec![];
    let last_def = last_def(&code);
    let mut temp = 0;
    let mut lvntable = LVNTable::new();

    for (line, c) in code.into_iter().enumerate() {
        match c {
            Code::Label { .. } => new_block.push(c),
            Code::Instruction(ins) => {
                let (dest, val) = lvntable.extract_dest_val(&ins);
                if let Some(dest) = dest {
                    let new_dest = if last_def[line] {
                        dest.clone()
                    } else {
                        temp += 1;
                        format!("_lvn{}_{}", temp, dest)
                    };
                    //handle special case where calls, alloc, ptradd, etc. should not be tabulated
                    // if !is_value(&ins) {
                    //     new_block.push(Code::Instruction(lvntable.produce_new_ins(
                    //         ins,
                    //         new_dest.clone(),
                    //         val,
                    //     )));
                    //     let num = lvntable.get_var(&new_dest);
                    //     lvntable.bind_var_to_num(&dest, num);
                    //     continue;
                    // }

                    new_block.push(Code::Instruction(lvntable.produce_new_ins(
                        ins,
                        new_dest.clone(),
                        val,
                    )));
                    let num = lvntable.get_var(&new_dest);
                    lvntable.bind_var_to_num(&dest, num);
                } else {
                    new_block.push(Code::Instruction(lvntable.produce_new_ins(
                        ins,
                        "".into(),
                        val,
                    )))
                }
            }
        }
    }
    new_block
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum LVNLiteral {
    /// Integers
    Int(i64),
    /// Booleans
    Bool(bool),
    /// Floating Points
    Float(String),
    /// UTF-16 Characters
    Char(char),
}

impl LVNLiteral {
    fn from_literal(t: &Type, l: Literal) -> Self {
        match l {
            Literal::Int(n) => match t {
                Type::Int => LVNLiteral::Int(n),
                Type::Float => LVNLiteral::Float(n.to_string()),
                _ => unreachable!("Expected int or float"),
            },
            Literal::Bool(b) => LVNLiteral::Bool(b),
            Literal::Float(f) => LVNLiteral::Float(f.to_string()),
            Literal::Char(ch) => LVNLiteral::Char(ch),
        }
    }
    fn to_literal(&self) -> Literal {
        match self {
            LVNLiteral::Int(n) => Literal::Int(*n),
            LVNLiteral::Bool(b) => Literal::Bool(*b),
            LVNLiteral::Float(f) => Literal::Float(f.parse().unwrap()),
            LVNLiteral::Char(ch) => Literal::Char(*ch),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum OP {
    Const(LVNLiteral),
    Value(ValueOps),
    Effect,
}

type NumberedVal = (OP, Vec<u32>);

struct LVNTable {
    //"cloud"
    var_num: HashMap<String, u32>,
    //maps value to number
    value_num: HashMap<NumberedVal, u32>,
    //maps numbering to canonical name
    num_canonical: HashMap<u32, String>,
    //next number
    num: u32,
}

impl LVNTable {
    fn new() -> Self {
        LVNTable {
            var_num: HashMap::new(),
            value_num: HashMap::new(),
            num_canonical: HashMap::new(),
            num: 0,
        }
    }
    ///safely retrieves num associated with var [name] by creating a new entry if [name] has not been tabulated
    fn get_var(&mut self, name: &String) -> u32 {
        match self.var_num.get(name) {
            Some(num) => *num,
            None => {
                self.num += 1;
                self.var_num.insert(name.clone(), self.num);
                self.num_canonical.insert(self.num, name.clone());
                self.num
            }
        }
    }

    ///adds a new value [exp] stored to [canonical_name]
    fn new_value(&mut self, exp: &NumberedVal, canonical_name: &String) {
        self.num += 1;
        self.value_num.insert(exp.clone(), self.num);
        self.var_num.insert(canonical_name.clone(), self.num);
        self.num_canonical.insert(self.num, canonical_name.clone());
    }

    ///binds variable to a table entry
    fn bind_var_to_num(&mut self, canonical_name: &String, num: u32) {
        self.var_num.insert(canonical_name.clone(), num);
    }

    ///find the number assigned to [val], returns [None] if not previously computed
    fn find_val(&self, val: &NumberedVal) -> Option<u32> {
        self.value_num.get(val).copied()
    }

    ///maps args to numbered form
    fn map_args(&mut self, args: &Vec<String>) -> Vec<u32> {
        args.iter().map(|x| self.get_var(x)).collect()
    }

    ///given instruction [ins], extracts dest (if one exists) and numbered args
    fn extract_dest_val(&mut self, ins: &Instruction) -> (Option<String>, NumberedVal) {
        match ins {
            Instruction::Constant {
                dest,
                value,
                const_type,
                ..
            } => (
                Some(dest.clone()),
                (
                    OP::Const(LVNLiteral::from_literal(const_type, value.clone())),
                    vec![],
                ),
            ),

            Instruction::Value { args, dest, op, .. } => (
                Some(dest.clone()),
                (OP::Value(op.clone()), self.map_args(args)),
            ),
            Instruction::Effect { args, .. } => (None, (OP::Effect, self.map_args(&args))),
        }
    }

    ///given old instruction [ins] and destination variable [dest], emit an [Instruction] that corresponds to computing [exp] using table values
    ///and updates the table if necessary
    fn produce_new_ins(&mut self, ins: Instruction, dest: String, exp: NumberedVal) -> Instruction {
        match ins {
            Instruction::Constant {
                pos, const_type, ..
            } => {
                if let Some(num) = self.find_val(&exp) {
                    self.bind_var_to_num(&dest, num)
                } else {
                    self.new_value(&exp, &dest);
                }
                Instruction::Constant {
                    dest,
                    value: match exp.0 {
                        OP::Const(l) => l.to_literal(),
                        _ => panic!("Expected constant"),
                    },
                    op: ConstOps::Const,
                    pos,
                    const_type,
                }
            }
            Instruction::Value {
                funcs,
                labels,
                op,
                pos,
                op_type,
                ..
            } => {
                let table_entry = self.find_val(&exp);
                // if the value is already in the table, emit an normal instruction
                //handles special case where calls, alloc, ptradd, etc. should not be tabulated
                if table_entry.is_none() || !is_value(&op) {
                    let res = Instruction::Value {
                        dest: dest.clone(),
                        args: exp
                            .1
                            .iter()
                            .map(|x| self.num_canonical.get(x).unwrap().clone())
                            .collect(),
                        funcs,
                        labels,
                        op,
                        pos,
                        op_type: op_type.clone(),
                    };
                    self.new_value(&exp, &dest);
                    res
                }
                //otherwise, emit the instruction as normal
                else {
                    let num = table_entry.unwrap();
                    let res = Instruction::Value {
                        dest: dest.clone(),
                        args: vec![self.num_canonical.get(&num).unwrap().clone()],
                        funcs,
                        labels,
                        op: ValueOps::Id,
                        pos,
                        op_type: op_type.clone(),
                    };
                    self.bind_var_to_num(&dest, num);
                    res
                }
            }
            Instruction::Effect {
                funcs,
                labels,
                op,
                pos,
                ..
            } => Instruction::Effect {
                args: exp
                    .1
                    .iter()
                    .map(|x| self.num_canonical.get(x).unwrap().clone())
                    .collect(),
                funcs,
                labels,
                op,
                pos,
            },
        }
    }
}

fn last_def(code: &Vec<Code>) -> Vec<bool> {
    let mut is_last_def = vec![false; code.len()];
    let mut defs = HashSet::new();
    for (idx, c) in code.iter().enumerate().rev() {
        if let Some(var) = c.defs() {
            if !defs.contains(&var) {
                is_last_def[idx] = true;
                defs.insert(var);
            }
        }
    }
    is_last_def
}

/// returns true if [op] performs an operation that can be a tabulated value
fn is_value(op: &ValueOps) -> bool {
    match op {
        ValueOps::Call | ValueOps::Alloc | ValueOps::Load | ValueOps::PtrAdd => false,
        _ => true,
    }
}
