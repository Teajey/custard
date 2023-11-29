use std::{
    collections::{hash_map::Values, HashMap},
    sync::{Arc, Mutex},
};

use camino::{Utf8Path, Utf8PathBuf};

use crate::fs::{self, path_has_extensions};

use super::FrontmatterFile;

pub struct Keeper {
    inner: HashMap<Utf8PathBuf, FrontmatterFile>,
}

#[derive(Debug, thiserror::Error)]
pub enum NewKeeperError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to load frontmatter file: {0}")]
    ReadFrontmatterFromPath(#[from] super::ReadFromPathError),
}

impl Keeper {
    pub fn new(path: &Utf8Path) -> Result<Self, NewKeeperError> {
        let markdown_fps = fs::filepaths_with_extensions(path, &["md"])?
            .into_iter()
            .map(|path| -> Result<_, super::ReadFromPathError> {
                let md = FrontmatterFile::read_from_path(&path)?;

                Ok((path, md))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Keeper {
            inner: markdown_fps,
        })
    }

    pub fn files(&self) -> Values<'_, Utf8PathBuf, FrontmatterFile> {
        self.inner.values()
    }
}

#[derive(Clone)]
pub struct ArcMutex(pub Arc<Mutex<Keeper>>);

impl ArcMutex {
    pub fn new(keeper: Keeper) -> Self {
        Self(Arc::new(Mutex::new(keeper)))
    }
}

impl Keeper {
    fn process_rename_event(&mut self, path: &Utf8Path) {
        let was_removed = self.inner.remove(path).is_some();
        if !was_removed {
            let file = match FrontmatterFile::read_from_path(path) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Couldn't load file ({path:?}) after Create event: {err}");
                    return;
                }
            };
            self.inner.insert(path.to_owned(), file);
        }
    }

    fn process_edit_event(&mut self, path: &Utf8Path) {
        let Some(file) = self.inner.get_mut(path) else {
            eprintln!("Couldn't find ({path:?}) in Edit event.");
            return;
        };
        let new_file = match FrontmatterFile::read_from_path(path) {
            Ok(new_file) => new_file,
            Err(err) => {
                eprintln!("Couldn't load file ({path:?}) after Edit event: {err}");
                return;
            }
        };
        *file = new_file;
    }

    fn process_removal_event(&mut self, path: &Utf8Path) {
        let was_removed = self.inner.remove(path).is_some();
        if !was_removed {
            eprintln!("Couldn't find ({path:?}) in Remove event..");
        }
    }

    fn process_create_event(&mut self, path: &Utf8Path) {
        if self.inner.contains_key(path) {
            eprintln!(
                "A Create event occurred for a path ({path:?}) but it already exists in memory."
            );
            return;
        }
        let new_file = match FrontmatterFile::read_from_path(path) {
            Ok(new_file) => new_file,
            Err(err) => {
                eprintln!("Couldn't load file ({path:?}) during Create event: {err}");
                return;
            }
        };
        self.inner.insert(path.to_owned(), new_file);
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
                let path = match Utf8PathBuf::try_from(path.clone()) {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Event filepath ({path:?}) was not UTF-8: {err}\n\nNon-UTF-8 paths not supported.");
                        return;
                    }
                };
                if !path_has_extensions(&path, &["md"]) {
                    return;
                }
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

#[cfg(test)]
mod test {
    use std::io::Write;

    use camino::Utf8PathBuf;
    use notify::{EventHandler, RecursiveMode, Watcher};

    use super::{ArcMutex, Keeper};

    struct TestFile {
        path: Utf8PathBuf,
    }

    impl TestFile {
        fn generate(&self) -> std::io::Result<()> {
            let _ = std::fs::File::create(&self.path)?;
            Ok(())
        }

        fn write<T: std::fmt::Display>(&self, str: T) -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&self.path)?;

            write!(file, "{str}")?;

            Ok(())
        }

        fn delete(&self) -> std::io::Result<()> {
            std::fs::remove_file(&self.path)
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            if self.path.exists() {
                std::fs::remove_file(&self.path).unwrap();
            }
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn file_monitoring() {
        let test_file_name = "test.md";
        let wd = Utf8PathBuf::try_from(std::env::temp_dir()).unwrap();
        let test_file_path = wd.join(test_file_name);
        let test_file = TestFile {
            path: test_file_path,
        };
        let keeper = ArcMutex::new(Keeper::new(&wd).unwrap());

        let (tx, rx) = std::sync::mpsc::channel();

        let mut keeper_mut = keeper.clone();
        let tx_clone = tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                match event {
                    Ok(event) => {
                        keeper_mut.handle_event(Ok(event.clone()));
                        tx_clone.send(Ok(event)).unwrap();
                    }
                    Err(err) => {
                        keeper_mut.handle_event(Err(err));
                        tx_clone.send(Err(())).unwrap();
                    }
                }
            })
            .unwrap();

        watcher
            .watch(wd.as_std_path(), RecursiveMode::NonRecursive)
            .unwrap();

        {
            let keeper = keeper.0.as_ref().lock().unwrap();
            let file = keeper.files().find(|file| file.name() == test_file_name);
            assert!(file.is_none());
        }

        test_file.generate().unwrap();

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Create(notify::event::CreateKind::File),
            event.kind,
            "Expecting a file create event"
        );

        let first_line = "Just call me Mark!\n";
        test_file.write(first_line).unwrap();

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Create(notify::event::CreateKind::File),
            event.kind,
            "Expecting a file create event"
        );

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content
            )),
            event.kind,
            "Expecting a content modification event"
        );

        {
            let keeper = keeper.0.as_ref().lock().unwrap();
            let file = keeper
                .files()
                .find(|file| file.name() == test_file_name)
                .expect("Keeper should have file now");
            assert_eq!(first_line, file.body);
        }

        let second_line = "I'm a markdown file!\n";
        test_file.write(second_line).unwrap();

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Create(notify::event::CreateKind::File),
            event.kind,
            "Expecting a create event"
        );

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content
            )),
            event.kind,
            "Expecting a content modification event"
        );

        {
            let keeper = keeper.0.as_ref().lock().unwrap();
            let file = keeper
                .files()
                .find(|file| file.name() == test_file_name)
                .unwrap();
            assert_eq!([first_line, second_line].join(""), file.body);
        }

        test_file.delete().unwrap();

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Create(notify::event::CreateKind::File),
            event.kind,
            "Expecting a create event"
        );

        let event = rx.recv().unwrap().unwrap();
        pretty_assertions::assert_eq!(
            notify::EventKind::Remove(notify::event::RemoveKind::File),
            event.kind,
            "Expecting a file delete event"
        );

        {
            let keeper = keeper.0.as_ref().lock().unwrap();
            let file = keeper.files().find(|file| file.name() == test_file_name);
            assert!(file.is_none());
        }
    }
}