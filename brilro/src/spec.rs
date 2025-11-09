use crate::parser::ast::*;

pub struct Trace<'a> {
    func: &'a str,
    trace: Vec<usize>,
}

impl<'a> Trace<'a> {
    pub fn parse_from_str(s: &'a str) -> Self {
        let mut parts = s.split(":");
        let name = parts.next().unwrap();
        let trace = parts
            .next()
            .unwrap()
            .split(",")
            .map(|s| s.parse().unwrap())
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
        if f.name != trace.func {
            continue;
        }

        let end_label_name = format!("__trace_end_{tid}");
        let end_label = Instruction::Label {
            label: end_label_name.clone(),
            span: None,
        };
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
                .map(|i| f.instrs[*i].clone())
                .filter_map(|i| match i {
                    Instruction::Effect {
                        op: EffectOp::Jmp, ..
                    } => None,
                    Instruction::Effect {
                        op: EffectOp::Br,
                        args,
                        funcs,
                        ..
                    } => Some(Instruction::Effect {
                        op: EffectOp::Guard,
                        args,
                        funcs,
                        labels: vec![abort_label_name.clone()],
                        span: None,
                    }),
                    Instruction::Label { .. } => None,
                    insn => Some(insn),
                }),
        )
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

        let &end_pos = trace.trace.last().unwrap();
        let &start_pos = trace.trace.first().unwrap();
        to_insert.push((end_pos + 1, vec![end_label]));
        to_insert.push((start_pos, new_insns));
    }
    let mut pre = vec![0; f.instrs.len()];
    for (pos, insns) in to_insert {
        let acc: usize = pre.iter().take(pos + 1).sum();
        pre[pos] += insns.len();
        f.instrs.splice(pos + acc..pos + acc, insns);
    }
}
