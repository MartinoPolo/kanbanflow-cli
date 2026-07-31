use clap::Parser;

/// CLI for KanbanFlow — scaffold only, no commands implemented yet.
#[derive(Parser)]
#[command(name = "kf", version, about, long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!("kf — KanbanFlow CLI (scaffold, no commands implemented yet)");
}
