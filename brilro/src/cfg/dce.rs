use std::collections::{HashMap, HashSet};

use crate::parser::ast::{EffectOp, Instruction};

use super::analysis::{BasicBlock, Cfg};

impl BasicBlock {
    /// Returns true on eliminating something.
    fn eliminate_dead_code(&mut self) -> bool {
        let mut dead = HashSet::new();
        let mut maybe_dead: HashMap<&String, usize> = HashMap::new();
        for (i, insn) in self.instrs.iter().enumerate() {
            // Remove used insns
            let args = match insn {
                Instruction::Constant { .. } => vec![],
                Instruction::Value { args, .. } => args.clone(),
                Instruction::Effect { args, .. } => args.clone(),
                Instruction::Label { .. } => vec![],
            };
            for arg in args {
                maybe_dead.remove(&arg);
            }

            // Add newly created argument.
            if let Instruction::Constant { dest, .. } | Instruction::Value { dest, .. } = insn {
                if maybe_dead.contains_key(dest) {
                    dead.insert(maybe_dead[dest]);
                }
                maybe_dead.insert(dest, i);
            }
        }

        let mut eliminated_something = false;
        let new_instrs = self
            .instrs
            .iter()
            .enumerate()
            .filter_map(|(i, insn)| {
                if dead.contains(&i) {
                    eliminated_something = true;
                    None
                } else {
                    Some(insn.clone())
                }
            })
            .collect();
        self.instrs = new_instrs;
        eliminated_something
    }

    pub fn dce(&mut self) {
        while self.eliminate_dead_code() {}
    }
}

impl Cfg {
    /// Basic block dce is nice, but a function global dce is also kind of needed.
    pub fn dce(&mut self) {
        loop {
            let assigned =
                self.blocks
                    .iter()
                    .flat_map(|block| {
                        block.instrs.iter().filter_map(|insn| match insn {
                            Instruction::Constant { dest, .. }
                            | Instruction::Value { dest, .. } => Some(dest.clone()),
                            Instruction::Effect { .. } | Instruction::Label { .. } => None,
                        })
                    })
                    .collect::<HashSet<String>>();
            let used = self
                .blocks
                .iter()
                .flat_map(|block| {
                    block
                        .instrs
                        .iter()
                        .filter_map(|insn| match insn {
                            Instruction::Effect { args, op, .. } => {
                                if matches!(op, EffectOp::Set) {
                                    Some(args[1..].to_vec())
                                } else {
                                    Some(args.clone())
                                }
                            }
                            Instruction::Value { args, .. } => Some(args.clone()),
                            Instruction::Label { .. } | Instruction::Constant { .. } => None,
                        })
                        .flatten()
                })
                .collect::<HashSet<String>>();

            let mut removed_insn = false;
            let unused: HashSet<_> = assigned.difference(&used).collect();
            for block in self.blocks.iter_mut() {
                block.instrs.retain(|i| match i {
                    Instruction::Value { dest, .. } | Instruction::Constant { dest, .. } => {
                        if unused.contains(dest) {
                            removed_insn = true;
                        }
                        !unused.contains(dest)
                    }
                    Instruction::Effect { args, op, .. } => {
                        if matches!(*op, EffectOp::Set) {
                            if unused.contains(&args[0]) {
                                removed_insn = true;
                            }
                            !unused.contains(&args[0])
                        } else {
                            true
                        }
                    }
                    Instruction::Label { .. } => true,
                });
            }

            let mut block_removed_insn = true;
            while block_removed_insn {
                block_removed_insn = false;
                for block in self.blocks.iter_mut() {
                    block_removed_insn |= block.eliminate_dead_code();
                    removed_insn |= block_removed_insn;
                }
            }
            if !removed_insn {
                break;
            }
        }
    }
}
