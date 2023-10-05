use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
struct MarkdownFile {
    frontmatter: Option<serde_yaml::Mapping>,
    body: String,
}

fn markdown_filepaths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for f in std::fs::read_dir(dir)? {
        let p = f?.path();
        if let Some(ext) = p.extension() {
            let str_ext = ext
                .to_str()
                .ok_or_else(|| anyhow!("Couldn't parse file extension as UTF-8 string"))?;
            if str_ext == "md" {
                files.push(p);
            }
        }
    }
    Ok(files)
}

fn run() -> Result<()> {
    if let Some(wd) = std::env::args().nth(1) {
        std::env::set_current_dir(wd)?;
    }

    let current_dir = std::env::current_dir()?;
    let markdown_fps = markdown_filepaths(&current_dir)?;
    let markdown_files = markdown_fps
        .into_iter()
        .map(|path| {
            let md = std::fs::read_to_string(&path)?;

            if !md.starts_with("---\n") {
                let md = MarkdownFile {
                    frontmatter: None,
                    body: md,
                };
                return Ok((path, md));
            }

            let [_, frontmatter, body] = md.splitn(3, "---\n").collect::<Vec<_>>()[..] else {
                let md = MarkdownFile {
                    frontmatter: None,
                    body: md,
                };
                return Ok((path, md));
            };
            dbg!(frontmatter, body);

            let frontmatter = serde_yaml::from_str(frontmatter)?;

            let md = MarkdownFile {
                frontmatter: Some(frontmatter),
                body: body.to_owned(),
            };

            Ok((path, md))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    dbg!(markdown_files);

    // Automatically select the best implementation for your platform.
    let mut watcher = notify::recommended_watcher(|res| match res {
        Ok(event) => println!("event: {event:?}"),
        Err(e) => println!("watch error: {e:?}"),
    })?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&current_dir, RecursiveMode::NonRecursive)?;

    Ok(())
}
