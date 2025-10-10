use brilro::{
    cfg::{
        analysis::{BasicBlock, Cfg},
        data_flow::{ReachingDefinitions, ShimmedCfg},
        dominator::DominatorTree,
        ssa,
    },
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
    Lvn,
    LvnDce,
    ReachingDefs,
    Dominator,
    ToSsa,
    FromSsa,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cfg" => Ok(Mode::Cfg),
            "rotate" => Ok(Mode::Rotate),
            "dce" => Ok(Mode::Dce),
            "lvn" => Ok(Mode::Lvn),
            "lvn-dce" => Ok(Mode::LvnDce),
            "reaching-defs" => Ok(Mode::ReachingDefs),
            "dom" => Ok(Mode::Dominator),
            "to-ssa" => Ok(Mode::ToSsa),
            "from-ssa" => Ok(Mode::FromSsa),
            _ => Err("unrecognized mode".to_string()),
        }
    }
}

#[derive(FromArgs)]
/// A funny little tool to rotate bril programs. Here rotate mean to take the last line of a
/// function and put it at the beginning.
///
/// There is additional functionality to print out CFGs of bril functions in the graphviz DOT
/// language and do various compiler optimizations.
struct Request {
    /// select what to do with the program, one of: "cfg", "rotate", "dce", "lvn", "lvn-dce",
    /// "reading-defs", "to-ssa"
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
    let mut prog: Program = match serde_json::from_str(&input) {
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
        Mode::Dce => run_dce(prog),
        Mode::Lvn => run_opt(prog, BasicBlock::lvn),
        Mode::LvnDce => {
            apply_to_all_blocks(&mut prog, BasicBlock::lvn);
            run_dce(prog)
        }
        Mode::ReachingDefs => run_reaching_defs(prog, cfg_fun),
        Mode::Dominator => run_dom(prog, cfg_fun),
        Mode::ToSsa => run_to_ssa(prog),
        Mode::FromSsa => run_from_ssa(prog),
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

fn apply_to_all_blocks<F>(prog: &mut Program, f: F)
where
    F: Fn(&mut BasicBlock),
{
    let new_functions = prog.functions.iter().map(|fun| {
        let mut cfg = Cfg::from_function(fun);
        cfg.apply_to_blocks(&f);
        cfg.function()
    });
    prog.functions = new_functions.collect();
}

fn run_to_ssa(mut prog: Program) -> Result<ExitCode, String> {
    for f in &mut prog.functions {
        let cfg = Cfg::from_function(f);
        let cfg = ssa::to_ssa(&cfg, f);
        *f = cfg.function();
    }
    println!("{}", serde_json::to_string_pretty(&prog).unwrap());
    Ok(ExitCode::SUCCESS)
}

fn run_from_ssa(mut prog: Program) -> Result<ExitCode, String> {
    for f in &mut prog.functions {
        let cfg = Cfg::from_function(f);
        let cfg = ssa::from_ssa(&cfg);
        *f = cfg.function();
    }
    println!("{}", serde_json::to_string_pretty(&prog).unwrap());
    Ok(ExitCode::SUCCESS)
}

fn run_dom(prog: Program, cfg_fun: String) -> Result<ExitCode, String> {
    let cfg = get_cfg(prog, cfg_fun)?;
    let dom = DominatorTree::from_cfg(&cfg);
    if dom.dominators_correct() {
        println!("dominators correct");
        Ok(ExitCode::SUCCESS)
    } else {
        Err("dominators incorrect, bug somewhere!!!".into())
    }
}

fn run_reaching_defs(prog: Program, cfg_fun: String) -> Result<ExitCode, String> {
    let cfg = get_cfg(prog, cfg_fun)?;
    let mut shimmed: ShimmedCfg<ReachingDefinitions> = ShimmedCfg::from_cfg(&cfg);
    shimmed.solve();
    shimmed.print_outsets();
    Ok(ExitCode::SUCCESS)
}

fn run_dce(mut prog: Program) -> Result<ExitCode, String> {
    for fun in prog.functions.iter_mut() {
        let mut cfg = Cfg::from_function(fun);
        cfg.dce();
        *fun = cfg.function();
    }
    let mutated_prog = serde_json::to_string_pretty(&prog).unwrap();
    println!("{mutated_prog}");
    Ok(ExitCode::SUCCESS)
}

fn run_opt<F>(mut prog: Program, f: F) -> Result<ExitCode, String>
where
    F: Fn(&mut BasicBlock),
{
    apply_to_all_blocks(&mut prog, f);
    let mutated_prog = serde_json::to_string_pretty(&prog).unwrap();
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
