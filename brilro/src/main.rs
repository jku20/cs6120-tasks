use brilro::{cfg::analysis::Cfg, parser::ast::Program};
use std::{
    io::{self, Read, Write},
    process::{Command, ExitCode, Stdio},
    time::SystemTime,
};

use argh::FromArgs;

#[derive(FromArgs)]
/// A funny little tool to rotate bril programs. Here rotate mean to take the last line of a
/// function and put it at the beginning.
///
/// There is additional functionality to print out CFGs of bril functions in the graphviz DOT
/// language.
struct Request {
    /// print the given function's CFG and do not rotate any programs.
    #[argh(option)]
    cfg_of: Option<String>,
}

fn main() -> ExitCode {
    let req: Request = argh::from_env();

    let mut stdin = io::stdin().lock();
    let mut input = String::new();
    let res = stdin.read_to_string(&mut input);
    if let Err(e) = res {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let prog: Program = match serde_json::from_str(&input) {
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
        Ok(json) => json,
    };

    if let Some(cfg_fun) = req.cfg_of {
        run_cfg(prog, cfg_fun)
    } else {
        run_rotate(prog)
    }
}

fn run_cfg(prog: Program, cfg_fun: String) -> ExitCode {
    let matching_funs = prog
        .functions
        .iter()
        .filter(|f| f.name == cfg_fun)
        .collect::<Vec<_>>();
    if let [f] = matching_funs[..] {
        let cfg = Cfg::from_function(f);
        println!("{}", cfg.as_dot());
        ExitCode::SUCCESS
    } else if matching_funs.is_empty() {
        eprintln!("error: no function with name {cfg_fun}");
        ExitCode::FAILURE
    } else {
        eprintln!("error: more than one function with name {cfg_fun}");
        ExitCode::FAILURE
    }
}

fn run_rotate(mut prog: Program) -> ExitCode {
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
    let now = SystemTime::now();
    while now.elapsed().unwrap().as_millis() < 100 {
        let output = c.try_wait().unwrap();
        if let Some(output) = output {
            return output.success();
        }
    }
    let _ = c.kill();
    let _ = c.wait();
    false
}
