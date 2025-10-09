use std::collections::HashMap;

use crate::parser::ast::{ConstOps, EffectOp, Instruction, Literal, Type, ValueOp};

use super::analysis::BasicBlock;

type ValueNum = usize;

#[derive(Debug, Clone)]
struct ValueInfo {
    src: String,
    value: AbstractValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AbstractValue {
    Constant {
        op: ConstOps,
        ty: Type,
        value: Literal,
    },
    Value {
        op: ValueOp,
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
            EffectOp::Call | EffectOp::Print | EffectOp::Nop | EffectOp::Set => false,
            EffectOp::Jmp => true,
            EffectOp::Br => true,
            EffectOp::Ret => true,
        },
        Instruction::Constant { .. } | Instruction::Value { .. } | Instruction::Label { .. } => {
            false
        }
    }
}

impl BasicBlock {
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
                    let dest = match info.values().find(|v| {
                        v.src == *dest
                            && !matches!(
                                v,
                                ValueInfo {
                                    value: AbstractValue::Opaque { .. },
                                    ..
                                }
                            )
                    }) {
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
        let mut sorted_vals: Vec<_> = last_dest.into_iter().collect();
        sorted_vals.sort_by(|a, b| a.0.cmp(&b.0));
        for (dest, (fresh, ty)) in sorted_vals {
            new_instrs.push(Instruction::Value {
                op: ValueOp::Id,
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
