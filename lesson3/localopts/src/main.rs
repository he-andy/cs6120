use bril_rs::load_program;
use clap::Parser;
use localopts::{analysis, cfg, lvn, tdce};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    dce: bool,
    #[arg(long)]
    lvn: bool,
    #[arg(long)]
    livevar: bool,
    #[arg(long)]
    debug_cfg: bool,
}
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let mut prog = load_program();
    let args = Args::parse();

    if args.lvn {
        prog = lvn::lvn(prog);
    }
    if args.dce {
        prog = tdce::local_pass(prog);
        prog = tdce::global_pass(prog);
    }

    if args.livevar {
        analysis::live_variable_debug(&prog, args.debug_cfg);
        return;
    }

    println!("{}", prog);
}
