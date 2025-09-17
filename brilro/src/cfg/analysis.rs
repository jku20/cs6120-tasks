use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Display,
};

use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Basic blocks are identified by the line they start.
    pub start: usize,
    pub name: Option<String>,
    pub instrs: Vec<Instruction>,
    pub flows_to: Vec<usize>,
}

type ValueNum = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AbstractValue {
    Constant {
        op: ConstOps,
        ty: Type,
        value: Literal,
    },
    Value {
        op: ValueOps,
        ty: Type,
        args: Vec<ValueNum>,
        funcs: Vec<String>,
        labels: Vec<String>,
    },
    Opaque {
        var: String,
    },
}

impl AbstractValue {
    fn from_instruction(
        insn: &Instruction,
        lvn: &HashMap<String, ValueNum>,
        last_dest: &HashMap<String, (String, Type)>,
    ) -> Result<Self, String> {
        match insn.clone() {
            Instruction::Constant { op, ty, value, .. } => Ok(Self::Constant { op, ty, value }),
            Instruction::Value {
                op,
                ty,
                args,
                funcs,
                labels,
                ..
            } => Ok(Self::Value {
                op,
                ty,
                args: args
                    .into_iter()
                    .map(|v| {
                        let real_v = last_dest.get(&v).map(|x| x.0.clone()).unwrap_or(v);
                        lvn[&real_v]
                    })
                    .collect(),
                funcs,
                labels,
            }),
            Instruction::Effect { .. } | Instruction::Label { .. } => {
                Err("instruction not of value or constant type.".to_string())
            }
        }
    }

    fn canonicalize(
        &mut self,
        _lvn: &HashMap<String, ValueNum>,
        _info: &HashMap<ValueNum, ValueInfo>,
    ) {
    }
}

fn is_terminator(insn: &Instruction) -> bool {
    match insn {
        Instruction::Effect { op, .. } => match op {
            EffectOps::Call | EffectOps::Print | EffectOps::Nop => false,
            EffectOps::Jmp => true,
            EffectOps::Br => true,
            EffectOps::Ret => true,
        },
        Instruction::Constant { .. } | Instruction::Value { .. } | Instruction::Label { .. } => {
            false
        }
    }
}

#[derive(Debug, Clone)]
struct ValueInfo {
    src: String,
    value: AbstractValue,
}

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

    fn replace_insn_args(
        insn: &mut Instruction,
        lvn: &mut HashMap<String, ValueNum>,
        info: &mut HashMap<ValueNum, ValueInfo>,
        next_num: &mut usize,
        last_dest: &HashMap<String, (String, Type)>,
    ) {
        match insn {
            Instruction::Constant { .. } | Instruction::Label { .. } => {}
            Instruction::Value { args, .. } | Instruction::Effect { args, .. } => {
                let new_args = args.clone().into_iter().map(|s| {
                    // For variables in previous blocks or function args, they might not be in the
                    // table.
                    let real_s = match &last_dest.get(&s) {
                        Some(s) => &s.0,
                        None => &s,
                    };
                    // If lvn doesn't contain this, the assignment must have happened in the past
                    // which is represented by an abstract value which is opaque. It will compare
                    // equal to other opaque types with the same variable.
                    if !lvn.contains_key(real_s) {
                        lvn.insert(real_s.clone(), *next_num);
                        let vi = ValueInfo {
                            src: real_s.clone(),
                            value: AbstractValue::Opaque {
                                var: real_s.clone(),
                            },
                        };
                        info.insert(*next_num, vi);
                        *next_num += 1;
                    }
                    info[&lvn[real_s]].src.clone()
                });
                args.clear();
                args.extend(new_args);
            }
        }
    }

    fn canonicalize_values(&mut self) {
        let mut next_num = 0;
        let mut fresh_idx = 0;
        let mut lvn = HashMap::new();
        let mut info = HashMap::new();
        let mut new_instrs = vec![];
        let mut last_dest = HashMap::new();
        for insn in &self.instrs {
            let mut new_insn = insn.clone();
            Self::replace_insn_args(
                &mut new_insn,
                &mut lvn,
                &mut info,
                &mut next_num,
                &last_dest,
            );
            match insn {
                i @ Instruction::Constant { dest, ty, .. }
                | i @ Instruction::Value { dest, ty, .. } => {
                    let mut abstr = AbstractValue::from_instruction(i, &lvn, &last_dest).unwrap();
                    let dest = match info.values().find(|v| v.src == *dest) {
                        Some(_) => {
                            let fresh = format!("__brilro_fresh{fresh_idx}");
                            fresh_idx += 1;
                            last_dest.insert(dest.clone(), (fresh.clone(), ty.clone()));
                            match &mut new_insn {
                                Instruction::Effect { .. } | Instruction::Label { .. } => {}
                                Instruction::Constant { dest, .. }
                                | Instruction::Value { dest, .. } => {
                                    dest.clear();
                                    dest.push_str(&fresh);
                                }
                            }
                            fresh
                        }
                        None => dest.clone(),
                    };
                    abstr.canonicalize(&lvn, &info);
                    let mut found = false;
                    for (&k, v) in &info {
                        if v.value == abstr {
                            lvn.insert(dest.clone(), k);
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        lvn.insert(dest.clone(), next_num);
                        let vi = ValueInfo {
                            src: dest.clone(),
                            value: abstr,
                        };
                        info.insert(next_num, vi);
                        next_num += 1;
                    }
                }
                _ => {}
            }
            new_instrs.push(new_insn);
        }

        let maybe_append = new_instrs.pop_if(|p| is_terminator(p));
        for (dest, (fresh, ty)) in last_dest {
            new_instrs.push(Instruction::Value {
                op: ValueOps::Id,
                dest,
                ty,
                args: vec![fresh],
                funcs: vec![],
                labels: vec![],
                span: None,
            });
        }
        if let Some(insn) = maybe_append {
            new_instrs.push(insn);
        }
        self.instrs = new_instrs;
    }

    pub fn lvn(&mut self) {
        self.canonicalize_values()
    }
}

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

#[derive(Debug)]
pub struct Cfg {
    original_function: Function,

    /// In sorted order by `start`
    blocks: Vec<BasicBlock>,
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
        blocks.sort_unstable();
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
}
