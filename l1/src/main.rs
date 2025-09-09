use brilro::parser::ast::Program;
use std::{
    io::{self, Read, Write},
    process::{Command, ExitCode, Stdio},
};

fn main() -> ExitCode {
    let mut stdin = io::stdin().lock();
    let mut input = String::new();
    let res = stdin.read_to_string(&mut input);
    if let Err(e) = res {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let mut prog: Program = match serde_json::from_str(&input) {
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
        Ok(json) => json,
    };

    rotate_functions(&mut prog);
    while !brili_says_it_runs(&prog) {
        rotate_functions(&mut prog);
    }

    let out = serde_json::to_string_pretty(&prog).unwrap();
    println!("{out}");

    ExitCode::SUCCESS
}

fn rotate_functions(prog: &mut Program) {
    for ref mut fun in &mut prog.functions {
        fun.instrs.rotate_right(1);
    }
}

fn brili_says_it_runs(p: &Program) -> bool {
    let prog_str = serde_json::to_string(p).unwrap();
    let mut c = Command::new("brili")
        .stdin(Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = c.stdin.take().unwrap();
    stdin.write_all(prog_str.as_bytes()).unwrap();
    drop(stdin);
    let output = c.wait().unwrap();
    output.success()
}
