use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use bril_rs::{Argument, Code, Instruction};

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
    pub phi: HashMap<String, (String, Vec<(String, String)>)>,
    pub instructions: Vec<Code>,
    pub defs: HashSet<String>,
    pub uses: HashSet<String>,
    pub vartype: HashMap<String, bril_rs::Type>,
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

    pub fn as_code_block(&self) -> Vec<Code> {
        let mut code = Vec::new();
        if let Some(label) = &self.label {
            code.push(Code::Label {
                label: label.clone(),
                pos: None,
            });
        }
        for (canonical, (phi_dest, phi_source)) in self.phi.iter() {
            let args = phi_source
                .iter()
                .map(|(var, _)| var.clone())
                .collect::<Vec<_>>();
            let labels = phi_source
                .iter()
                .map(|(_, label)| label.clone())
                .collect::<Vec<_>>();
            code.push(Code::Instruction(Instruction::Value {
                args: args,
                dest: phi_dest.clone(),
                funcs: vec![],
                labels: labels,
                op: bril_rs::ValueOps::Phi,
                pos: None,
                op_type: self.vartype.get(canonical).unwrap().clone(),
            }));
        }
        code.extend(self.instructions.clone());
        code
    }

    fn value_type(code: &Code) -> Option<(String, bril_rs::Type)> {
        match code {
            Code::Label { .. } => None,
            Code::Instruction(ins) => match ins {
                Instruction::Constant {
                    dest, const_type, ..
                } => Some((dest.clone(), const_type.clone())),
                Instruction::Value { dest, op_type, .. } => Some((dest.clone(), op_type.clone())),
                Instruction::Effect { .. } => None,
            },
        }
    }
}

pub fn code_to_bb(stmts: Vec<Code>) -> Vec<BasicBlock> {
    code_to_bb_extra_args(stmts, &vec![])
}

pub fn code_to_bb_extra_args(stmts: Vec<Code>, init_types: &Vec<Argument>) -> Vec<BasicBlock> {
    let mut vartype = HashMap::new();
    let mut blocks = Vec::new();
    let mut block = BasicBlock {
        label: None,
        phi: HashMap::new(),
        instructions: Vec::new(),
        defs: HashSet::new(),
        uses: HashSet::new(),
        vartype: HashMap::new(),
    };
    for stmt in stmts {
        match stmt.control_flow() {
            CF::Jump(_) | CF::Branch(_, _) | CF::Return => {
                if let Some((var, t)) = BasicBlock::value_type(&stmt) {
                    vartype.insert(var, t);
                }
                block.instructions.push(stmt);

                (block.uses, block.defs) = block.uses_and_defs();
                blocks.push(block);
                block = BasicBlock {
                    label: None,
                    phi: HashMap::new(),
                    instructions: Vec::new(),
                    defs: HashSet::new(),
                    uses: HashSet::new(),
                    vartype: HashMap::new(),
                };
            }
            CF::Normal => {
                if let Some((var, t)) = BasicBlock::value_type(&stmt) {
                    vartype.insert(var, t);
                }
                block.instructions.push(stmt);
            }
            CF::Label(label) => {
                (block.uses, block.defs) = block.uses_and_defs();
                blocks.push(block);
                block = BasicBlock {
                    label: Some(label),
                    phi: HashMap::new(),
                    instructions: vec![],
                    defs: HashSet::new(),
                    uses: HashSet::new(),
                    vartype: HashMap::new(),
                }
            }
        }
    }
    blocks.push(block);

    for arg in init_types {
        vartype.insert(arg.name.clone(), arg.arg_type.clone());
    }

    for block in blocks.iter_mut() {
        block.vartype = vartype.clone();
    }
    blocks
        .into_iter()
        .filter(|x| !x.instructions.is_empty() | x.label.is_some())
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
        match self.instructions.last() {
            Some(ins) => ins.control_flow(),
            None => match &self.label {
                Some(label) => CF::Label(label.clone()),
                None => panic!(), //empty block
            },
        }
    }

    fn is_label(&self) -> Option<String> {
        self.label.clone()
    }
}

impl Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ins in &self.as_code_block() {
            writeln!(f, "  {}", ins)?;
        }
        Ok(())
    }
}
