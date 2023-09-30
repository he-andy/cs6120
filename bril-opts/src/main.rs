use bril_opts::analysis::live_variable_analysis;
use bril_opts::cfg::graph_from_function;
use bril_opts::dominator::{dom_tree, dominator_analyis};
use bril_opts::ssa::to_ssa;
use bril_opts::{analysis, lvn, tdce};
use bril_rs::load_program;
use clap::Parser;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    dce: bool,
    #[arg(long)]
    lvn: bool,
    #[arg(long)]
    liveness: bool,
    #[arg(long)]
    dom: bool,
    #[arg(long)]
    cfg: bool,
    #[arg(long)]
    ssa: bool,
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

    if args.liveness {
        analysis::live_variable_debug(&prog);
    }
    if args.ssa {
        conv_to_ssa(&mut prog);
        println!("{}", prog);
    }
    if args.liveness || args.dom || args.cfg || args.ssa {
        for func in &prog.functions {
            function_analysis(&args, func);
        }
    } else {
        println!("{}", prog);
    }
}

fn conv_to_ssa(prog: &mut bril_rs::Program) {
    for func in &mut prog.functions {
        let cfg = graph_from_function(func);
        let mut ssa = to_ssa(&cfg);
        //ssa.debug_cfg();
        func.instrs = ssa.flatten();
    }
}

fn function_analysis(args: &Args, func: &bril_rs::Function) {
    let mut cfg = graph_from_function(func);
    cfg.label();

    // if args.ssa {
    //     let mut ssa = to_ssa(&cfg);
    //     if args.cfg {
    //         ssa.debug_cfg();
    //     }
    // }

    let (entry, exit) = live_variable_analysis(&cfg);
    if args.cfg {
        println!("@{} CFG", func.name);
        cfg.debug_cfg();
        println!("");
    }

    if args.liveness {
        println!("@{} Liveness Analysis", func.name);
        for i in 0..cfg.graph.node_count() {
            println!(
                "{:?}:",
                cfg.graph
                    .node_weight(petgraph::graph::NodeIndex::from(i as u32))
                    .unwrap()
                    .label
                    .clone()
                    .unwrap()
            );
            println!("Entry: {:?}", entry[i]);
            println!("Exit: {:?}", exit[i]);
            println!("");
        }
    }

    if args.dom {
        let (dominators, dom_frontier) = dominator_analyis(&cfg);
        println!("@{} Dominator Analysis", func.name);
        for i in 0..cfg.graph.node_count() {
            println!(
                "{:?}:",
                cfg.graph
                    .node_weight(NodeIndex::from(i as u32))
                    .unwrap()
                    .label
                    .clone()
                    .unwrap()
            );
            println!(
                "Dominators: {:?}",
                match dominators.dominators(NodeIndex::from(i as u32)) {
                    Some(x) => x
                        .into_iter()
                        .map(|x| cfg.graph.node_weight(x).unwrap().label.clone().unwrap())
                        .collect::<Vec<_>>(),
                    None => vec![],
                }
            );
            println!(
                "Dominance Frontier: {:?}",
                dom_frontier[i]
                    .iter()
                    .map(|x| cfg.graph.node_weight(*x).unwrap().label.clone().unwrap())
                    .collect::<Vec<_>>()
            );
            println!("");
        }

        println!("Dominance Tree:");
        let dom_tree = dom_tree(&dominators, &cfg);
        println!("{:?}", Dot::with_config(&dom_tree, &[Config::EdgeNoLabel]));
    }
}
