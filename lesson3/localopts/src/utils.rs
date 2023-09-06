use bril_rs::{Code, Instruction};

pub trait CFGNode {
    fn uses(&self) -> Vec<String>;
    fn defs(&self) -> Option<String>;
    fn control_flow(&self) -> CF;
}

pub enum CF {
    Normal,
    Label(String),
    Branch(String, String),
    Jump(String),
    r#Return,
}

impl CFGNode for Instruction {
    fn uses(&self) -> Vec<String> {
        match self {
            Instruction::Constant { .. } => vec![],
            Instruction::Value { args, .. } => args.clone(),
            Instruction::Effect { args, .. } => args.clone(),
        }
    }

    fn defs(&self) -> Option<String> {
        match self {
            Instruction::Constant { dest, .. } => Some(dest.clone()),
            Instruction::Value { dest, .. } => Some(dest.clone()),
            Instruction::Effect { .. } => None,
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
}

impl CFGNode for Code {
    fn uses(&self) -> Vec<String> {
        match self {
            Code::Label { .. } => vec![],
            Code::Instruction(ins) => ins.uses(),
        }
    }

    fn defs(&self) -> Option<String> {
        match self {
            Code::Label { .. } => None,
            Code::Instruction(ins) => ins.defs(),
        }
    }
    fn control_flow(&self) -> CF {
        match self {
            Code::Label { label, .. } => CF::Label(label.clone()),
            Code::Instruction(ins) => ins.control_flow(),
        }
    }
}

pub fn basic_blocks<T: CFGNode>(stmts: Vec<T>) -> Vec<Vec<T>> {
    let mut blocks = Vec::new();
    let mut block = Vec::new();
    for stmt in stmts {
        match stmt.control_flow() {
            CF::Jump(_) | CF::Branch(_, _) | CF::Return => {
                block.push(stmt);
                blocks.push(block);
                block = Vec::new();
            }
            CF::Normal => {
                block.push(stmt);
            }
            CF::Label(_) => {
                blocks.push(block);
                block = vec![stmt]
            }
        }
    }
    blocks.push(block);
    blocks.into_iter().filter(|x| !x.is_empty()).collect()
}
