use std::fs;
use std::path::PathBuf;

use apidiff::{diff, spec};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "apidiff",
    about = "Diffs two OpenAPI specs and reports breaking changes"
)]
struct Cli {
    old: PathBuf,
    new: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let old_json = fs::read_to_string(&cli.old)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", cli.old.display()))?;
    let new_json = fs::read_to_string(&cli.new)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", cli.new.display()))?;

    let old = spec::parse_spec(&old_json)?;
    let new = spec::parse_spec(&new_json)?;

    let changes = diff::find_breaking_changes(&old, &new);
    if changes.is_empty() {
        println!("no breaking changes");
        Ok(())
    } else {
        println!("{} breaking change(s):", changes.len());
        for c in &changes {
            println!("  {}", c.describe());
        }
        std::process::exit(1);
    }
}
