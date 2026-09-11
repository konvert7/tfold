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
}

fn main() {
    let args = Args::parse();
    match tfold::run(
        &args.root,
        args.budget as f64,
        args.include_tests,
        &args.exclude,
        args.since.as_deref(),
    ) {
        Ok(map) => println!("{map}"),
        Err(error) => {
            eprintln!("tfold: {error}");
            std::process::exit(2);
        }
    }
}
