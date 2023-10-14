mod frontmatter_file;
mod fs;

use std::collections::HashMap;

use anyhow::Result;
use notify::{RecursiveMode, Watcher};

use frontmatter_file::FrontmatterFile;

fn run() -> Result<()> {
    if let Some(wd) = std::env::args().nth(1) {
        std::env::set_current_dir(wd)?;
    }

    let current_dir = std::env::current_dir()?;
    let markdown_fps = fs::filepaths_with_extensions(&current_dir, &["md"])?;
    let markdown_files = markdown_fps
        .into_iter()
        .map(|path| {
            let string = std::fs::read_to_string(&path)?;

            let md = FrontmatterFile::from_string(string)?;

            Ok((path, md))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    let markdown_files = frontmatter_file::Map::new(markdown_files);

    // Automatically select the best implementation for your platform.
    let mut watcher = notify::recommended_watcher(markdown_files)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&current_dir, RecursiveMode::NonRecursive)?;

    std::thread::park();

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
