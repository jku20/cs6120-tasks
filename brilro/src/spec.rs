use crate::parser::ast::*;

pub struct Trace<'a> {
    func: &'a str,
    trace: Vec<(usize, bool)>,
}

impl<'a> Trace<'a> {
    pub fn parse_from_str(s: &'a str) -> Self {
        let mut parts = s.split(":");
        let name = parts.next().unwrap();
        let trace = parts
            .next()
            .unwrap()
            .split(";")
            .map(|s| {
                let s: Vec<&str> = s.split(",").collect();
                (s[0].parse().unwrap(), s[1].parse().unwrap())
            })
            .collect();
        Self { func: name, trace }
    }
}

pub fn speculate_from_traces(prog: &mut Program, traces: &[Trace]) {
    for f in &mut prog.functions {
        spec_fun(f, traces);
    }
}

fn spec_fun(f: &mut Function, traces: &[Trace]) {
    let mut to_insert = vec![];
    for (tid, trace) in traces.iter().enumerate() {
        let mut cond_number = 0;
        if f.name != trace.func {
            continue;
        }

        let mut end_label_name = format!("__trace_end_{tid}");
        let mut end_label = Some(Instruction::Label {
            label: end_label_name.clone(),
            span: None,
        });
        let abort_label_name = format!("__trace_abort_{tid}");
        let new_insns: Vec<Instruction> = [Instruction::Effect {
            op: EffectOp::Speculate,
            args: vec![],
            funcs: vec![],
            labels: vec![],
            span: None,
        }]
        .into_iter()
        .chain(
            trace
                .trace
                .iter()
                .enumerate()
                .map(|(j, (i, b))| (j, (f.instrs[*i].clone(), b)))
                .filter_map(|(j, (insn, &b))| match insn {
                    Instruction::Effect {
                        op: EffectOp::Jmp,
                        labels,
                        ..
                    } => {
                        if j == trace.trace.len() - 1 {
                            end_label = None;
                            end_label_name = labels[0].clone();
                            None
                        } else {
                            None
                        }
                    }
                    Instruction::Effect {
                        op: EffectOp::Br,
                        args,
                        funcs,
                        labels,
                        ..
                    } => {
                        if j == trace.trace.len() - 1 {
                            end_label = None;
                            end_label_name = if b {
                                labels[0].clone()
                            } else {
                                labels[1].clone()
                            };
                        }
                        if b {
                            Some(vec![Instruction::Effect {
                                op: EffectOp::Guard,
                                args,
                                funcs,
                                labels: vec![abort_label_name.clone()],
                                span: None,
                            }])
                        } else {
                            Some(vec![
                                Instruction::Value {
                                    op: ValueOp::Not,
                                    dest: {
                                        cond_number += 1;
                                        format!("__trace_cond_{tid}_{cond_number}")
                                    },
                                    ty: Type::Bool,
                                    args,
                                    funcs: vec![],
                                    labels: vec![],
                                    span: None,
                                },
                                Instruction::Effect {
                                    op: EffectOp::Guard,
                                    args: vec![format!("__trace_cond_{tid}_{cond_number}")],
                                    funcs,
                                    labels: vec![abort_label_name.clone()],
                                    span: None,
                                },
                            ])
                        }
                    }
                    Instruction::Label { .. } => None,
                    insn => Some(vec![insn]),
                })
                .flatten(),
        )
        .collect::<Vec<_>>()
        .into_iter()
        .chain([
            Instruction::Effect {
                op: EffectOp::Commit,
                args: vec![],
                funcs: vec![],
                labels: vec![],
                span: None,
            },
            Instruction::Effect {
                op: EffectOp::Jmp,
                args: vec![],
                funcs: vec![],
                labels: vec![end_label_name.clone()],
                span: None,
            },
            Instruction::Label {
                label: abort_label_name.clone(),
                span: None,
            },
        ])
        .collect();

        let end_pos = trace.trace.last().unwrap().0;
        let start_pos = trace.trace.first().unwrap().0;
        if let Some(end_label) = end_label {
            to_insert.push((end_pos + 1, vec![end_label]));
        }
        to_insert.push((start_pos, new_insns));
    }
    let mut pre = vec![0; f.instrs.len()];
    for (pos, insns) in to_insert {
        let acc: usize = pre.iter().take(pos + 1).sum();
        pre[pos] += insns.len();
        f.instrs.splice(pos + acc..pos + acc, insns);
    }
}
