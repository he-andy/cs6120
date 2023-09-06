use bril_rs::load_program;
use clap::Parser;
use localopts::tdce;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    dce: bool,
}
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let mut prog = load_program();
    let args = Args::parse();

    if args.dce {
        prog = tdce::global_pass(prog)
    }
    println!("{}", prog);
}
