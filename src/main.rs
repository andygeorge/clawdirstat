mod cli;
mod scanner;
mod ui;

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let args = cli::parse();

    let target_dir = args
        .dir
        .unwrap_or_else(|| env::current_dir().expect("could not get cwd"));

    let mut root = scanner::scan(&target_dir)?;
    scanner::sort_by_size(&mut root.children);

    ui::app::run(root, args.count)?;

    Ok(())
}
