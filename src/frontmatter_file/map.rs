use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::fs::path_has_extensions;

use super::FrontmatterFile;

pub struct Map {
    map: HashMap<PathBuf, FrontmatterFile>,
}

impl Map {
    pub fn new(map: HashMap<PathBuf, FrontmatterFile>) -> Self {
        Self { map }
    }

    fn process_rename_event(&mut self, path: &Path) {
        let was_removed = self.map.remove(path).is_some();
        if !was_removed {
            let new_content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read file ({path:?}) after a Rename event: {err}");
                    return;
                }
            };
            let new_data = match FrontmatterFile::from_string(new_content) {
                Ok(new_data) => new_data,
                Err(err) => {
                    eprintln!("Couldn't read frontmatter during Create event: {err}");
                    return;
                }
            };
            self.map.insert(path.to_owned(), new_data);
        }
    }

    fn process_edit_event(&mut self, path: &Path) {
        let new_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read file ({path:?}) after a Modify event: {err}");
                return;
            }
        };
        let Some(data) = self.map.get_mut(path) else {
            eprintln!("Tried to get data for a path ({path:?}) that doesn't exist in memory.");
            return;
        };
        let new_data = match FrontmatterFile::from_string(new_content) {
            Ok(new_data) => new_data,
            Err(err) => {
                eprintln!("Couldn't read frontmatter during Modify event: {err}");
                return;
            }
        };
        *data = new_data;
    }

    fn process_removal_event(&mut self, path: &Path) {
        let was_removed = self.map.remove(path).is_some();
        if !was_removed {
            eprintln!(
                "Recieved a removal event for a path ({path:?}) that didn't exist in memory."
            );
        }
    }

    fn process_create_event(&mut self, path: &Path) {
        let new_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read file ({path:?}) after a Create event: {err}");
                return;
            }
        };
        if self.map.contains_key(path) {
            eprintln!(
                "A Create event occurred for a path ({path:?}) but it already exists in memory."
            );
            return;
        }
        let new_data = match FrontmatterFile::from_string(new_content) {
            Ok(new_data) => new_data,
            Err(err) => {
                eprintln!("Couldn't read frontmatter during Create event: {err}");
                return;
            }
        };
        self.map.insert(path.to_owned(), new_data);
    }
}

impl notify::EventHandler for Map {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(notify::Event {
                kind,
                paths,
                attrs: _,
            }) => {
                let path = paths.first().expect("event must have at least one path");
                if !path_has_extensions(path, &["md"]) {
                    return;
                }
                match kind {
                    notify::EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::Any,
                    )) => {
                        self.process_rename_event(path);
                    }
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(
                        notify::event::DataChange::Content,
                    )) => {
                        self.process_edit_event(path);
                    }
                    notify::EventKind::Remove(notify::event::RemoveKind::File) => {
                        self.process_removal_event(path);
                    }
                    notify::EventKind::Create(notify::event::CreateKind::File) => {
                        self.process_create_event(path);
                    }
                    event => println!("unhandled watch event: {event:?}"),
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        }

        println!("\nUpdated data:\n{:#?}", self.map);
    }
}
