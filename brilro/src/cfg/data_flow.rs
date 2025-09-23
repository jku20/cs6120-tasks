use std::collections::BTreeSet;

use crate::parser::ast::*;

use super::analysis::{BasicBlock, Cfg};

#[derive(Debug)]
pub struct Info<S> {
    block: BasicBlock,
    inset: S,
    outset: S,
}

pub trait Flow {
    type Set: Clone + Eq + std::fmt::Debug;

    fn transfer(block: &mut Info<Self::Set>);
    fn merge(a: &Self::Set, b: &Self::Set) -> Self::Set;
    fn inital() -> Self::Set;

    fn string_of_set(s: &Self::Set) -> String;
}

pub struct ReachingDefinitions {}

impl Flow for ReachingDefinitions {
    type Set = BTreeSet<(usize, String)>;

    fn transfer(block: &mut Info<Self::Set>) {
        let mut outset = block.inset.clone();
        for insn in &block.block.instrs {
            match insn {
                Instruction::Constant { dest, .. } | Instruction::Value { dest, .. } => {
                    if let Some(p) = outset.iter().find(|(_, s)| s == dest) {
                        let p = p.clone();
                        outset.remove(&p);
                    }
                    outset.insert((block.block.start, dest.clone()));
                }
                Instruction::Effect { .. } | Instruction::Label { .. } => {}
            }
        }
        block.outset = outset;
    }

    fn merge(a: &Self::Set, b: &Self::Set) -> Self::Set {
        a.union(b).cloned().collect()
    }

    fn inital() -> Self::Set {
        BTreeSet::new()
    }

    fn string_of_set(s: &Self::Set) -> String {
        let mut out = "".to_string();
        for (k, v) in s {
            if !out.is_empty() {
                out = format!("{out}, ({k}: {v})");
            } else {
                out = format!("({k}: {v})");
            }
        }
        out
    }
}

pub struct ShimmedCfg<T: Flow> {
    pub(super) blocks: Vec<Info<T::Set>>,
}

impl<T: Flow> ShimmedCfg<T> {
    pub fn from_cfg(cfg: &Cfg) -> Self {
        Self {
            blocks: cfg
                .blocks
                .iter()
                .map(|b| Info {
                    block: b.clone(),
                    inset: T::inital(),
                    outset: T::inital(),
                })
                .collect(),
        }
    }

    pub fn solve(&mut self) {
        let mut worklist: Vec<usize> = (0..self.blocks.len()).collect();
        while let Some(b) = worklist.pop() {
            let parents = self
                .blocks
                .iter()
                .filter(|i| i.block.flows_to.contains(&self.blocks[b].block.start));
            let merged = parents.fold(T::inital(), |acc, p| T::merge(&p.outset, &acc));
            self.blocks[b].inset = merged;
            let last_out = self.blocks[b].outset.clone();
            T::transfer(&mut self.blocks[b]);
            if last_out != self.blocks[b].outset {
                for &s in &self.blocks[b].block.flows_to {
                    let to_push = self.blocks.iter().position(|p| p.block.start == s).unwrap();
                    worklist.push(to_push);
                }
            }
        }
    }

    pub fn print_outsets(&self) {
        for block in self.blocks.iter() {
            println!("{}: {}", block.block.start, T::string_of_set(&block.outset));
        }
    }
}
