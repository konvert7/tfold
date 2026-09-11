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

    /// Exclude paths matching a glob, repeatable
    #[arg(long, value_name = "GLOB", value_parser = tfold::collect::validate_exclude)]
    exclude: Vec<String>,

    /// Mark and rank up what changed since a git revision
    #[arg(long, value_name = "REF")]
    since: Option<String>,

    /// Count literal matches per file and spend the budget where they are
    #[arg(long, value_name = "PATTERN", value_parser = tfold::scan::validate_pattern)]
    grep: Option<String>,

    /// Match --grep regardless of case
    #[arg(short = 'i', long, requires = "grep")]
    ignore_case: bool,
}

fn main() {
    let args = Args::parse();
    let options = tfold::Options {
        budget: args.budget as f64,
        include_tests: args.include_tests,
        excludes: args.exclude,
        since: args.since,
        grep: args.grep,
        ignore_case: args.ignore_case,
    };
    match tfold::run(&args.root, &options) {
        Ok(map) => println!("{map}"),
        Err(error) => {
            eprintln!("tfold: {error}");
            std::process::exit(2);
        }
    }
}
