use std::{collections::BTreeMap, fmt::Display};

use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Basic blocks are identified by the line they start.
    pub start: usize,
    pub name: Option<String>,
    pub instrs: Vec<Instruction>,
    pub flows_to: Vec<usize>,
    pub pred: Vec<usize>,
}

impl BasicBlock {}

impl PartialEq for BasicBlock {
    fn eq(&self, other: &Self) -> bool {
        self.start.eq(&other.start)
    }
}
impl Eq for BasicBlock {}

impl PartialOrd for BasicBlock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BasicBlock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
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

#[derive(Debug, Clone)]
pub struct Cfg {
    pub(super) original_function: Function,

    /// In sorted order by `start`
    pub(super) blocks: Vec<BasicBlock>,
}

impl Cfg {
    pub fn from_function(f: &Function) -> Self {
        let base = f.clone();
        let original_function = f.clone();

        // Get labels to convert
        let mut line: BTreeMap<String, usize> = BTreeMap::new();
        for (i, insn) in base.instrs.iter().enumerate() {
            if let Instruction::Label { label, .. } = insn.clone() {
                line.insert(label, i);
            }
        }

        let mut instrs = vec![];
        let mut name = None;
        let mut start = 0;
        let mut blocks = vec![];
        for (i, insn) in base.instrs.iter().enumerate() {
            match insn {
                Instruction::Label { label, .. } => {
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
                            pred: vec![],
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
                        EffectOp::Jmp => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![line[&labels[0]]],
                                pred: vec![],
                            });
                            instrs = vec![];
                            start = i + 1;
                            name = None;
                        }
                        EffectOp::Br => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![line[&labels[0]], line[&labels[1]]],
                                pred: vec![],
                            });
                            instrs = vec![];
                            start = i + 1;
                            name = None;
                        }
                        EffectOp::Ret => {
                            blocks.push(BasicBlock {
                                start,
                                name: name.clone(),
                                instrs: instrs.clone(),
                                flows_to: vec![],
                                pred: vec![],
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
                pred: vec![],
            });
        }
        blocks.sort_unstable();

        let blocks = blocks
            .iter()
            .map(|block| BasicBlock {
                pred: blocks
                    .iter()
                    .filter_map(|i| {
                        if i.flows_to.contains(&block.start) {
                            Some(i.start)
                        } else {
                            None
                        }
                    })
                    .collect(),
                ..block.clone()
            })
            .collect();

        Cfg {
            blocks,
            original_function,
        }
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

    pub fn apply_to_blocks<F>(&mut self, f: F)
    where
        F: Fn(&mut BasicBlock),
    {
        for ref mut block in self.blocks.iter_mut() {
            f(block);
        }
    }

    pub fn function(&self) -> Function {
        let mut fun = self.original_function.clone();
        let new_instrs = self
            .blocks
            .clone()
            .into_iter()
            .flat_map(|b| b.instrs)
            .collect();
        fun.instrs = new_instrs;
        fun
    }

    /// Returns the block given the block start.
    ///
    /// Panics if start isn't found.
    pub fn block(&self, start: usize) -> &BasicBlock {
        self.blocks.iter().find(|p| p.start == start).unwrap()
    }

    /// Returns the block given the block start but as a mutable reference.
    ///
    /// Panics if start isn't found.
    pub fn block_mut(&mut self, start: usize) -> &mut BasicBlock {
        self.blocks.iter_mut().find(|p| p.start == start).unwrap()
    }
}
