use std::{collections::BTreeMap, fmt::Display};

use crate::parser::ast::*;

#[derive(Debug)]
pub struct BasicBlock {
    /// Basic blocks are identified by the line they start.
    pub start: usize,
    pub name: Option<String>,
    pub instrs: Vec<Instruction>,
    pub flows_to: Vec<usize>,
}

impl Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let strs: Vec<_> = self
            .instrs
            .iter()
            .map(|i| serde_json::to_string_pretty(i).unwrap())
            .collect();
        let s = strs.join("\n");
        write!(f, "{s}")
    }
}

#[derive(Debug)]
pub struct Cfg {
    blocks: Vec<BasicBlock>,
}

impl Cfg {
    pub fn from_function(f: &Function) -> Self {
        let base = f.clone();

        // Get labels to convert
        let mut line: BTreeMap<String, usize> = BTreeMap::new();
        for (i, insn) in base.instrs.iter().enumerate() {
            if let Instruction::Label { label } = insn.clone() {
                line.insert(label, i);
            }
        }

        let mut instrs = vec![];
        let mut name = None;
        let mut start = 0;
        let mut blocks = vec![];
        for (i, insn) in base.instrs.iter().enumerate() {
            match insn {
                Instruction::Label { label } => {
                    if instrs.is_empty() {
                        // This label is the first thing in the block.
                        name = Some(label.clone());
                        start = i;
                        instrs.push(insn.clone());
                    } else {
                        // This label means we must cut off the block and start a new one.
                        blocks.push(BasicBlock {
                            start,
                            name: name.clone(),
                            instrs: instrs.clone(),
                            flows_to: vec![i],
                        });

                        instrs = vec![insn.clone()];
                        start = i;
                        name = Some(label.clone());
                    }
                }
                Instruction::Effect { op, labels, .. } => {
                    // We finish a block including this insn
                    instrs.push(insn.clone());
                    match op {
                        EffectOps::Jmp => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![line[&labels[0]]],
                            });
                            instrs = vec![];
                            start = i + 1;
                            name = None;
                        }
                        EffectOps::Br => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![line[&labels[0]], line[&labels[1]]],
                            });
                            instrs = vec![];
                            start = i + 1;
                            name = None;
                        }
                        EffectOps::Ret => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![],
                            });
                            instrs = vec![];
                            start = i + 1;
                            name = None;
                        }
                        _ => {}
                    }
                }
                _ => {
                    instrs.push(insn.clone());
                }
            }
        }

        if !instrs.is_empty() {
            blocks.push(BasicBlock {
                start,
                name,
                instrs,
                flows_to: vec![],
            });
        }

        Cfg { blocks }
    }

    pub fn as_dot(&self) -> String {
        let mut header = "digraph cfg {".to_string();
        let mut strs: BTreeMap<usize, String> = BTreeMap::new();
        for b in &self.blocks {
            strs.insert(b.start, b.to_string());
        }

        for b in &self.blocks {
            for i in &b.flows_to {
                let n1 = b.to_string().replace("\"", "");
                let n2 = &strs[i].replace("\"", "");
                let line = format!("\n\"{}\" -> \"{}\"", n1, n2);
                header = format!("{}{}", header, line);
            }
        }
        format!("{}{}", header, "\n}")
    }
}
