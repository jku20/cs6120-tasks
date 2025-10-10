use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    cfg::lvn::is_terminator,
    parser::ast::{EffectOp, Function, Instruction, Type, ValueOp},
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
    pub stack: HashMap<String, Vec<String>>,
    pub name_nums: HashMap<String, usize>,
}

impl NameMaker {
    fn new() -> Self {
        Self {
            stack: HashMap::new(),
            name_nums: HashMap::new(),
        }
    }

    fn name(&mut self, name: &str) -> String {
        self.stack
            .entry(name.to_string())
            .or_insert_with(|| {
                let new_idx = self.name_nums.entry(name.to_string()).or_default();
                let name = format!("{name}{new_idx}");
                *new_idx += 1;
                vec![name]
            })
            .last()
            .unwrap()
            .clone()
    }

    fn push(&mut self, name: &str) {
        let new_idx = self.name_nums.entry(name.to_string()).or_default();
        let new_name = format!("{name}{new_idx}");
        *new_idx += 1;
        self.stack
            .entry(name.to_string())
            .or_default()
            .push(new_name);
    }
}

#[derive(Debug)]
struct Ssaifier<'a> {
    cfg: Cfg,
    defs: HashMap<&'a str, BTreeSet<(usize, Type)>>,
    doms: DominatorTree,
    phis: HashMap<usize, HashMap<&'a str, PhiNode>>,
    types: HashMap<String, Type>,
    func: Function,
}

impl<'a> Ssaifier<'a> {
    fn from_cfg_and_func(cfg: &'a Cfg, func: &Function) -> Self {
        let mut defs: HashMap<&str, BTreeSet<(usize, Type)>> = HashMap::new();
        let mut vars_defined: HashMap<usize, BTreeSet<&'a str>> = HashMap::new();
        let mut types = HashMap::new();
        for block in &cfg.blocks {
            for insn in &block.instrs {
                match insn {
                    Instruction::Constant { dest, ty, .. }
                    | Instruction::Value { dest, ty, .. } => {
                        defs.entry(dest).or_default().insert((block.start, *ty));
                        types.insert(dest.clone(), *ty);
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
            types,
            func: func.clone(),
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
        types: &mut HashMap<String, Type>,
        insn: &mut Instruction,
        phis: &mut HashMap<usize, HashMap<&'a str, PhiNode>>,
        block_start: usize,
        names: &mut NameMaker,
    ) {
        // Replace args
        match insn {
            Instruction::Value { args, .. } | Instruction::Effect { args, .. } => {
                *args = args
                    .iter()
                    .map(|n| {
                        eprintln!("arg: {block_start} with {names:?} replacing {n}");
                        let name = names.name(n);
                        eprintln!("replacing with {name}");
                        types.insert(name.clone(), types[n]);
                        name
                    })
                    .collect();
            }
            Instruction::Constant { .. } | Instruction::Label { .. } => {}
        }
        // Replace dest
        match insn {
            Instruction::Value { dest, op, ty, .. } => {
                eprintln!("dest: {block_start} with {names:?} replacing {dest}");
                names.push(dest);
                let name = names.name(dest);
                if matches!(op, ValueOp::Get) {
                    phis.get_mut(&block_start)
                        .unwrap()
                        .get_mut(&dest[..])
                        .unwrap()
                        .dest = name.clone();
                }
                types.insert(name.clone(), *ty);
                *dest = name;
            }
            Instruction::Constant { dest, ty, .. } => {
                eprintln!("dest: {block_start} with {names:?} replacing {dest}");
                names.push(dest);
                eprintln!("after: {block_start} with {names:?} replacing {dest}");
                let name = names.name(dest);
                types.insert(name.clone(), *ty);
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
            Self::replace_names(&mut self.types, insn, &mut self.phis, block_start, names);
        }
        for succ in &block.flows_to {
            if let Some(phis) = self.phis.get_mut(succ) {
                for (var, phi) in phis {
                    phi.args.insert(block_start, names.name(var));
                }
            }
        }
        for &domed in &self.doms.im_dom[&block_start].clone() {
            if domed != block_start && self.cfg.block(block_start).flows_to.contains(&domed) {
                eprintln!("dominating {}", domed);
                self.rename_block(domed, names);
            }
        }
        eprintln!("resetting stack");
        names.stack = old_stack;
    }

    fn rename(&mut self) {
        self.rename_block(0, &mut NameMaker::new());
    }

    fn add_sets(&mut self) {
        for sources in self.phis.values() {
            eprintln!("source: {:?}", sources);
            for (orig, phi) in sources {
                for (&block, set_arg) in &phi.args {
                    let instrs = &mut self.cfg.block_mut(block).instrs;
                    let idx = if is_terminator(instrs.last().unwrap()) {
                        instrs.len() - 1
                    } else {
                        instrs.len()
                    };
                    self.types
                        .insert(phi.dest.clone(), self.types[&orig.to_string()]);
                    self.types
                        .insert(set_arg.clone(), self.types[&orig.to_string()]);
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

    fn add_undef_to_block(&mut self, mut cur_defs: HashSet<String>, block_start: usize) {
        let block = self.cfg.block_mut(block_start);
        let mut num_inserted = 0;
        for (idx, insn) in block.instrs.clone().into_iter().enumerate() {
            match insn {
                Instruction::Label { .. } => {}
                Instruction::Constant { dest, .. } => {
                    cur_defs.insert(dest);
                }
                Instruction::Value { args, dest, .. } => {
                    for arg in &args {
                        if !cur_defs.contains(arg) {
                            block.instrs.insert(
                                idx + num_inserted,
                                Instruction::Value {
                                    op: ValueOp::Undef,
                                    dest: arg.clone(),
                                    ty: self.types[arg],
                                    args: vec![],
                                    funcs: vec![],
                                    labels: vec![],
                                    span: None,
                                },
                            );
                            num_inserted += 1;
                            cur_defs.insert(arg.clone());
                        }
                    }
                    cur_defs.insert(dest);
                }
                Instruction::Effect { args, op, .. } => {
                    for (i, arg) in args.iter().enumerate() {
                        if i == 0 && matches!(op, EffectOp::Set) {
                            continue;
                        }
                        if !cur_defs.contains(arg) {
                            block.instrs.insert(
                                idx + num_inserted,
                                Instruction::Value {
                                    op: ValueOp::Undef,
                                    dest: arg.clone(),
                                    ty: self.types[arg],
                                    args: vec![],
                                    funcs: vec![],
                                    labels: vec![],
                                    span: None,
                                },
                            );
                            num_inserted += 1;
                            cur_defs.insert(arg.clone());
                        }
                    }
                }
            }
        }
        for &domed in &self.doms.im_dom[&block_start].clone() {
            if domed != block_start && self.cfg.block(block_start).flows_to.contains(&domed) {
                self.add_undef_to_block(cur_defs.clone(), domed);
            }
        }
    }

    fn add_undefs(&mut self) {
        self.add_undef_to_block(self.func.args.iter().map(|a| a.name.clone()).collect(), 0);
    }

    fn cfg(self) -> Cfg {
        self.cfg
    }
}

pub fn to_ssa(cfg: &Cfg, func: &Function) -> Cfg {
    let mut ssaifier = Ssaifier::from_cfg_and_func(cfg, func);
    ssaifier.compute_phis();
    ssaifier.rename();
    ssaifier.add_sets();
    ssaifier.add_undefs();
    ssaifier.cfg()
}

pub fn from_ssa(cfg: &Cfg) -> Cfg {
    cfg.clone()
}
