use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Result, Value};
use std::collections::HashSet;
use std::{io, iter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Program {
    functions: Vec<Function>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    name: String,
    args: Vec<Args>,
    instrs: Vec<Instruction>,
}

type Instruction = Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Args {
    name: String,
    r#type: String,
}

fn main() -> Result<()> {
    let lines = io::stdin().lines();

    let buffer = lines
        .into_iter()
        .map(|x| match x {
            Ok(s) => s,
            _ => "".into(),
        })
        .collect::<Vec<String>>()
        .join("\n");

    let mut program: Program = serde_json::from_str(&buffer).expect("failed deserialize");

    for func in program.functions.iter_mut() {
        let mut chars = func.name.chars().collect::<HashSet<_>>();
        chars.extend("caled".chars());
        let new_chars = chars
            .into_iter()
            .map(|x| {
                format!(
                    r#"{{ "op": "const", "type": "char", "dest": "_fnname_{x}", "value": "{x}" }}"#
                )
            })
            .collect::<Vec<String>>();

        let args = format!("{}called", func.name)
            .chars()
            .map(|x| format!(r#""_fnname_{x}""#))
            .collect::<Vec<String>>()
            .join(",\n");

        let print_instr = format!(
            r#"{{
                "args": [
                    {args}
                ],
                "op": "print"
            }}"#,
        );

        let new_instrs = new_chars
            .into_iter()
            .chain(iter::once(print_instr))
            .map(|x| serde_json::from_str(&x).unwrap())
            .collect::<Vec<Value>>();

        func.instrs = new_instrs
            .into_iter()
            .chain(func.instrs.clone().into_iter())
            .collect();
    }
    println!("{}", json!(program).to_string());
    Ok(())
}
