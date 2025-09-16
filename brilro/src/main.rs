use brilro::{
    cfg::analysis::{BasicBlock, Cfg},
    parser::ast::Program,
};
use std::{
    io::{self, Read, Write},
    process::{Command, ExitCode, Stdio},
    str::FromStr,
    time::SystemTime,
};

use argh::FromArgs;

enum Mode {
    Cfg,
    Rotate,
    Dce,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cfg" => Ok(Mode::Cfg),
            "rotate" => Ok(Mode::Rotate),
            "dce" => Ok(Mode::Dce),
            _ => Err("unrecognized mode".to_string()),
        }
    }
}

#[derive(FromArgs)]
/// A funny little tool to rotate bril programs. Here rotate mean to take the last line of a
/// function and put it at the beginning.
///
/// There is additional functionality to print out CFGs of bril functions in the graphviz DOT
/// language.
struct Request {
    /// select what to do with the program, one of: "cfg", "rotate", "dce"
    #[argh(option, short = 'm')]
    mode: Mode,

    /// print the given function's CFG and do not rotate any programs.
    #[argh(option)]
    cfg_fun: Option<String>,
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

    let cfg_fun = if let Some(cfg_fun) = req.cfg_fun {
        cfg_fun
    } else {
        "main".to_string()
    };
    let res = match req.mode {
        Mode::Cfg => run_cfg(prog, cfg_fun),
        Mode::Rotate => run_rotate(prog),
        Mode::Dce => run_dce(prog, cfg_fun),
    };

    match res {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn get_cfg(prog: Program, cfg_fun: String) -> Result<Cfg, String> {
    let matching_funs = prog
        .functions
        .iter()
        .filter(|f| f.name == cfg_fun)
        .collect::<Vec<_>>();
    if let [f] = matching_funs[..] {
        Ok(Cfg::from_function(f))
    } else if matching_funs.is_empty() {
        Err(format!("no function with name {cfg_fun}"))
    } else {
        Err(format!("error: more than one function with name {cfg_fun}"))
    }
}

fn run_dce(prog: Program, cfg_fun: String) -> Result<ExitCode, String> {
    let mut cfg = get_cfg(prog, cfg_fun)?;
    cfg.apply_to_blocks(BasicBlock::dce);
    let mutated_prog = serde_json::to_string_pretty(&cfg.prog()).unwrap();
    println!("{mutated_prog}");
    Ok(ExitCode::SUCCESS)
}

fn run_cfg(prog: Program, cfg_fun: String) -> Result<ExitCode, String> {
    let cfg = get_cfg(prog, cfg_fun)?;
    println!("{}", cfg.as_dot());
    Ok(ExitCode::SUCCESS)
}

fn run_rotate(mut prog: Program) -> Result<ExitCode, String> {
    rotate_functions(&mut prog);
    while !brili_says_it_runs(&prog) {
        rotate_functions(&mut prog);
    }

    let out = serde_json::to_string_pretty(&prog).unwrap();
    println!("{out}");

    Ok(ExitCode::SUCCESS)
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
