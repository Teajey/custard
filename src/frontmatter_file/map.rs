use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{fs::path_has_extensions, utf8_filepath::UTF8FilePath};

use super::FrontmatterFile;

pub struct Map {
    pub inner: HashMap<UTF8FilePath, FrontmatterFile>,
}

#[derive(Clone)]
pub struct ArcMutex(pub Arc<Mutex<Map>>);

impl ArcMutex {
    pub fn new(map: HashMap<UTF8FilePath, FrontmatterFile>) -> Self {
        Self(Arc::new(Mutex::new(Map { inner: map })))
    }
}

impl Map {
    fn process_rename_event(&mut self, path: &UTF8FilePath) {
        let was_removed = self.inner.remove(path).is_some();
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
            self.inner.insert(path.clone(), new_data);
        }
    }

    fn process_edit_event(&mut self, path: &UTF8FilePath) {
        let new_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read file ({path:?}) after a Modify event: {err}");
                return;
            }
        };
        let Some(data) = self.inner.get_mut(path) else {
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

    fn process_removal_event(&mut self, path: &UTF8FilePath) {
        let was_removed = self.inner.remove(path).is_some();
        if !was_removed {
            eprintln!(
                "Recieved a removal event for a path ({path:?}) that didn't exist in memory."
            );
        }
    }

    fn process_create_event(&mut self, path: &UTF8FilePath) {
        let new_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read file ({path:?}) after a Create event: {err}");
                return;
            }
        };
        if self.inner.contains_key(path) {
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
        self.inner.insert(path.clone(), new_data);
    }
}

impl notify::EventHandler for ArcMutex {
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
                let path = match UTF8FilePath::try_from(path.clone()) {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Event filepath ({path:?}) was not UTF-8: {err}\n\nNon-UTF-8 paths not supported.");
                        return;
                    }
                };
                let mut map = match self.0.as_ref().lock() {
                    Ok(map) => map,
                    Err(err) => {
                        eprintln!("Failed to lock data map during notify event: {err}");
                        return;
                    }
                };
                match kind {
                    notify::EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::Any,
                    )) => {
                        map.process_rename_event(&path);
                    }
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(
                        notify::event::DataChange::Content,
                    )) => {
                        map.process_edit_event(&path);
                    }
                    notify::EventKind::Remove(notify::event::RemoveKind::File) => {
                        map.process_removal_event(&path);
                    }
                    notify::EventKind::Create(notify::event::CreateKind::File) => {
                        map.process_create_event(&path);
                    }
                    event => println!("unhandled watch event: {event:?}"),
                }
            }
            Err(e) => println!("watch error: {e:?}"),
        }
    }
}
