use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use notify::{RecursiveMode, Watcher};

#[derive(Debug)]
struct MarkdownData {
    frontmatter: Option<serde_yaml::Mapping>,
    body: String,
}

impl MarkdownData {
    fn from_string(string: String) -> Result<Self, serde_yaml::Error> {
        if !string.starts_with("---\n") {
            let md = MarkdownData {
                frontmatter: None,
                body: string,
            };
            return Ok(md);
        }

        let [_, frontmatter, body] = string.splitn(3, "---\n").collect::<Vec<_>>()[..] else {
            let md = MarkdownData {
                frontmatter: None,
                body: string,
            };
            return Ok(md);
        };

        let frontmatter = serde_yaml::from_str(frontmatter)?;

        Ok(MarkdownData {
            frontmatter: Some(frontmatter),
            body: body.to_owned(),
        })
    }
}

fn path_is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|ext| ext == "md")
}

fn markdown_filepaths(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    let path = entry.path();
                    if path_is_markdown(&path) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .transpose()
        })
        .collect()
}

struct DataSynchroniser {
    data: HashMap<PathBuf, MarkdownData>,
}

impl notify::EventHandler for DataSynchroniser {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(notify::Event {
                kind,
                paths,
                attrs: _,
            }) => {
                let path = paths.first().expect("event must have at least one path");
                if !path_is_markdown(path) {
                    return;
                }
                match kind {
                    notify::EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::Any,
                    )) => {
                        let was_removed = self.data.remove(path).is_some();
                        if !was_removed {
                            let new_content = match std::fs::read_to_string(path) {
                                Ok(content) => content,
                                Err(err) => {
                                    eprintln!(
                                        "Failed to read file ({path:?}) after a Rename event: {err}"
                                    );
                                    return;
                                }
                            };
                            let new_data = match MarkdownData::from_string(new_content) {
                                Ok(new_data) => new_data,
                                Err(err) => {
                                    eprintln!("Couldn't read markdown during Create event: {err}");
                                    return;
                                }
                            };
                            self.data.insert(path.clone(), new_data);
                        }
                    }
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(
                        notify::event::DataChange::Content,
                    )) => {
                        let new_content = match std::fs::read_to_string(path) {
                            Ok(content) => content,
                            Err(err) => {
                                eprintln!(
                                    "Failed to read file ({path:?}) after a Modify event: {err}"
                                );
                                return;
                            }
                        };
                        let Some(data) = self.data.get_mut(path) else {
                            eprintln!("Tried to get data for a path ({path:?}) that doesn't exist in memory.");
                            return;
                        };
                        let new_data = match MarkdownData::from_string(new_content) {
                            Ok(new_data) => new_data,
                            Err(err) => {
                                eprintln!("Couldn't read markdown during Modify event: {err}");
                                return;
                            }
                        };
                        *data = new_data;
                    }
                    notify::EventKind::Remove(notify::event::RemoveKind::File) => {
                        let was_removed = self.data.remove(path).is_some();
                        if !was_removed {
                            eprintln!("Recieved a removal event for a path ({path:?}) that didn't exist in memory.");
                        }
                    }
                    notify::EventKind::Create(notify::event::CreateKind::File) => {
                        let new_content = match std::fs::read_to_string(path) {
                            Ok(content) => content,
                            Err(err) => {
                                eprintln!(
                                    "Failed to read file ({path:?}) after a Create event: {err}"
                                );
                                return;
                            }
                        };
                        if self.data.contains_key(path) {
                            eprintln!(
                                "A Create event occurred for a path ({path:?}) but it already exists in memory."
                            );
                            return;
                        }
                        let new_data = match MarkdownData::from_string(new_content) {
                            Ok(new_data) => new_data,
                            Err(err) => {
                                eprintln!("Couldn't read markdown during Create event: {err}");
                                return;
                            }
                        };
                        self.data.insert(path.clone(), new_data);
                    }
                    event => println!("watch event: {event:?}"),
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        }

        println!("\nUpdated data:\n{:#?}", self.data);
    }
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
            let string = std::fs::read_to_string(&path)?;

            let md = MarkdownData::from_string(string)?;

            Ok((path, md))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    let synchroniser = DataSynchroniser {
        data: markdown_files,
    };

    // Automatically select the best implementation for your platform.
    let mut watcher = notify::recommended_watcher(synchroniser)?;

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
