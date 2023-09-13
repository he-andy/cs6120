use std::{collections::HashSet, hash::Hash, fmt::Display};

use bril_rs::{Code, Instruction};

pub trait CFGNode {
    fn uses(&self) -> HashSet<String>;
    fn defs(&self) -> HashSet<String>;
    fn control_flow(&self) -> CF;
    fn is_label(&self) -> Option<String>;
}

pub enum CF {
    Normal,
    Label(String),
    Branch(String, String),
    Jump(String),
    r#Return,
}

impl CFGNode for Instruction {
    fn uses(&self) -> HashSet<String> {
        match self {
            Instruction::Constant { .. } => HashSet::new(),
            Instruction::Value { args, .. } => args.iter().cloned().collect(),
            Instruction::Effect { args, .. } => args.iter().cloned().collect(),
        }
    }

    fn defs(&self) -> HashSet<String> {
        match self {
            Instruction::Constant { dest, .. } => HashSet::from([dest.clone()]),
            Instruction::Value { dest, .. } => HashSet::from([dest.clone()]),
            Instruction::Effect { .. } => HashSet::new(),
        }
    }

    fn control_flow(&self) -> CF {
        match self {
            Instruction::Constant { .. } => CF::Normal,
            Instruction::Value { .. } => CF::Normal,
            Instruction::Effect { labels, op, .. } => match op {
                bril_rs::EffectOps::Jump => CF::Jump(labels.first().unwrap().clone()),
                bril_rs::EffectOps::Branch => CF::Branch(
                    labels.first().unwrap().clone(),
                    labels.last().unwrap().clone(),
                ),
                bril_rs::EffectOps::Return => CF::Return,
                _ => CF::Normal,
            },
        }
    }
    fn is_label(&self) -> Option<String> {
        None
    }
}

impl CFGNode for Code {
    fn uses(&self) -> HashSet<String> {
        match self {
            Code::Label { .. } => HashSet::new(),
            Code::Instruction(ins) => ins.uses(),
        }
    }

    fn defs(&self) -> HashSet<String> {
        match self {
            Code::Label { .. } => HashSet::new(),
            Code::Instruction(ins) => ins.defs(),
        }
    }
    fn control_flow(&self) -> CF {
        match self {
            Code::Label { label, .. } => CF::Label(label.clone()),
            Code::Instruction(ins) => ins.control_flow(),
        }
    }
    fn is_label(&self) -> Option<String> {
        match self {
            Code::Label { label, .. } => Some(label.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub label: Option<String>,
    pub instructions: Vec<Code>,
    pub defs: HashSet<String>,
    pub uses: HashSet<String>,
}

impl BasicBlock {
    pub fn uses_and_defs(&self) -> (HashSet<String>, HashSet<String>) {
        let mut defs = HashSet::new();
        let mut uses = HashSet::new();
        for ins in &self.instructions {
            uses.extend(ins.uses().difference(&defs).into_iter().cloned());
            defs = defs.union(&ins.defs()).cloned().collect();
        }
        (uses, defs)
    }
}
pub fn basic_blocks(stmts: Vec<Code>) -> Vec<BasicBlock> {
    let mut blocks = Vec::new();
    let mut block = BasicBlock {
        label: None,
        instructions: Vec::new(),
        defs: HashSet::new(),
        uses: HashSet::new(),
    };
    for stmt in stmts {
        match stmt.control_flow() {
            CF::Jump(_) | CF::Branch(_, _) | CF::Return => {
                block.instructions.push(stmt);
                (block.uses, block.defs) = block.uses_and_defs();

                blocks.push(block);
                block = BasicBlock {
                    label: None,
                    instructions: Vec::new(),
                    defs: HashSet::new(),
                    uses: HashSet::new(),
                };
            }
            CF::Normal => {
                block.instructions.push(stmt);
            }
            CF::Label(label) => {
                (block.uses, block.defs) = block.uses_and_defs();

                blocks.push(block);
                block = BasicBlock {
                    label: Some(label),
                    instructions: vec![stmt],
                    defs: HashSet::new(),
                    uses: HashSet::new(),
                }
            }
        }
    }
    blocks.push(block);
    blocks
        .into_iter()
        .filter(|x| !x.instructions.is_empty())
        .collect() //remove empty
}

impl CFGNode for BasicBlock {
    fn uses(&self) -> HashSet<String> {
        self.uses.clone().into_iter().collect()
    }

    fn defs(&self) -> HashSet<String> {
        self.defs.clone().into_iter().collect()
    }

    fn control_flow(&self) -> CF {
        self.instructions.last().unwrap().control_flow()
    }

    fn is_label(&self) -> Option<String> {
        self.label.clone()
    }
}

impl Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ins in &self.instructions {
            writeln!(f, "  {}", ins)?;
        }
        Ok(())
    }
}
