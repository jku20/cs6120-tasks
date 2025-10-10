use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    cfg::lvn::is_terminator,
    parser::ast::{EffectOp, Instruction, Type, ValueOp},
};

use super::{analysis::Cfg, dominator::DominatorTree};

#[derive(Debug)]
struct PhiNode {
    dest: String,
    args: HashMap<usize, String>,
}

impl PhiNode {
    fn from_dest(dest: &str) -> Self {
        Self {
            dest: dest.to_string(),
            args: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct NameMaker {
    pub stack: HashMap<String, u32>,
}

impl NameMaker {
    fn new() -> Self {
        Self {
            stack: HashMap::new(),
        }
    }

    fn name(&mut self, name: &str) -> String {
        format!(
            "{}{}",
            name,
            self.stack.entry(name.to_string()).or_default()
        )
    }

    fn push(&mut self, name: &str) {
        *self.stack.entry(name.to_string()).or_default() += 1;
    }
}

#[derive(Debug)]
struct Ssaifier<'a> {
    cfg: Cfg,
    defs: HashMap<&'a str, BTreeSet<(usize, Type)>>,
    doms: DominatorTree,
    phis: HashMap<usize, HashMap<&'a str, PhiNode>>,
}

impl<'a> Ssaifier<'a> {
    fn from_cfg(cfg: &'a Cfg) -> Self {
        let mut defs: HashMap<&str, BTreeSet<(usize, Type)>> = HashMap::new();
        let mut vars_defined: HashMap<usize, BTreeSet<&'a str>> = HashMap::new();
        for block in &cfg.blocks {
            for insn in &block.instrs {
                match insn {
                    Instruction::Constant { dest, ty, .. }
                    | Instruction::Value { dest, ty, .. } => {
                        defs.entry(dest).or_default().insert((block.start, *ty));
                        vars_defined.entry(block.start).or_default().insert(dest);
                    }
                    Instruction::Effect { .. } | Instruction::Label { .. } => {}
                }
            }
        }
        Self {
            cfg: cfg.clone(),
            defs,
            doms: DominatorTree::from_cfg(cfg),
            phis: HashMap::new(),
        }
    }

    fn compute_phis(&mut self) {
        for (&var, defs) in &mut self.defs {
            let mut defs_with_maybe_mods = defs.clone();
            let mut new_defs = defs.clone();
            while let Some((def, ty)) = defs_with_maybe_mods.pop_last() {
                for &block in &self.doms.frontier[&def] {
                    if !self.phis.contains_key(&block) || !self.phis[&block].contains_key(var) {
                        self.phis
                            .entry(block)
                            .or_default()
                            .insert(var, PhiNode::from_dest(var));
                        let instrs = &mut self.cfg.block_mut(block).instrs;
                        let idx = if matches!(instrs[0], Instruction::Label { .. }) {
                            1
                        } else {
                            0
                        };
                        instrs.insert(
                            idx,
                            Instruction::Value {
                                op: ValueOp::Get,
                                dest: var.to_string(),
                                ty,
                                args: vec![],
                                funcs: vec![],
                                labels: vec![],
                                span: None,
                            },
                        );
                        defs_with_maybe_mods.insert((block, ty));
                        new_defs.insert((block, ty));
                    }
                }
            }
            *defs = new_defs;
        }
    }

    fn replace_names(
        insn: &mut Instruction,
        phis: &mut HashMap<usize, HashMap<&'a str, PhiNode>>,
        block_start: usize,
        names: &mut NameMaker,
    ) {
        // Replace args
        match insn {
            Instruction::Value { args, .. } | Instruction::Effect { args, .. } => {
                *args = args.iter().map(|n| names.name(n)).collect();
            }
            Instruction::Constant { .. } | Instruction::Label { .. } => {}
        }
        // Replace dest
        match insn {
            Instruction::Value { dest, op, .. } => {
                names.push(dest);
                let name = names.name(dest);
                if matches!(op, ValueOp::Get) {
                    phis.get_mut(&block_start)
                        .unwrap()
                        .get_mut(&dest[..])
                        .unwrap()
                        .dest = name.clone();
                }
                *dest = name;
            }
            Instruction::Constant { dest, .. } => {
                names.push(dest);
                *dest = names.name(dest);
            }
            Instruction::Effect { .. } | Instruction::Label { .. } => {}
        }
    }

    fn rename_block(&mut self, block_start: usize, names: &mut NameMaker) {
        eprintln!("renaming: {}", block_start);
        let block = self.cfg.block_mut(block_start);
        let old_stack = names.stack.clone();
        for insn in &mut block.instrs {
            Self::replace_names(insn, &mut self.phis, block_start, names);
        }
        for succ in &block.flows_to {
            if let Some(phis) = self.phis.get_mut(succ) {
                for (var, phi) in phis {
                    eprintln!("name: {}", names.name(var));
                    phi.args.insert(block_start, names.name(var));
                }
            }
        }
        for &domed in &self.doms.im_dom[&block_start].clone() {
            if domed != block_start && self.cfg.block(block_start).flows_to.contains(&domed) {
                self.rename_block(domed, names);
            }
        }
        names.stack = old_stack;
    }

    fn rename(&mut self) {
        self.rename_block(0, &mut NameMaker::new());
    }

    fn add_sets(&mut self) {
        for sources in self.phis.values() {
            eprintln!("source: {:?}", sources);
            for phi in sources.values() {
                for (&block, set_arg) in &phi.args {
                    let instrs = &mut self.cfg.block_mut(block).instrs;
                    let idx = if is_terminator(instrs.last().unwrap()) {
                        instrs.len() - 1
                    } else {
                        instrs.len()
                    };
                    instrs.insert(
                        idx,
                        Instruction::Effect {
                            op: EffectOp::Set,
                            args: vec![phi.dest.clone(), set_arg.clone()],
                            funcs: vec![],
                            labels: vec![],
                            span: None,
                        },
                    );
                }
            }
        }
    }

    fn cfg(self) -> Cfg {
        self.cfg
    }
}

pub fn to_ssa(cfg: &Cfg) -> Cfg {
    let mut ssaifier = Ssaifier::from_cfg(cfg);
    ssaifier.compute_phis();
    ssaifier.rename();
    ssaifier.add_sets();
    ssaifier.cfg()
}

pub fn from_ssa(cfg: &Cfg) -> Cfg {
    cfg.clone()
}
