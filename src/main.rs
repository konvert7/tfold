use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "tfold",
    about = "A repository structure map that fits a token budget"
)]
struct Args {
    /// Directory to map
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Token budget for the output
    #[arg(long, default_value_t = 800)]
    budget: usize,

    /// Include test files instead of collapsing them to a count
    #[arg(long)]
    include_tests: bool,
}

fn main() {
    let args = Args::parse();
    println!(
        "{}",
        tfold::run(&args.root, args.budget as f64, args.include_tests)
    );
}
