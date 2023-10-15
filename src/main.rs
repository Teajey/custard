mod frontmatter_file;
mod fs;

use std::collections::HashMap;

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing, Json, Router,
};
use notify::{RecursiveMode, Watcher};

use frontmatter_file::FrontmatterFile;
use serde::Deserialize;

async fn frontmatter_get_many(
    State(markdown_files): State<frontmatter_file::map::ArcMutex>,
) -> Result<Json<Vec<FrontmatterFile>>, StatusCode> {
    let map = markdown_files.0.as_ref();
    let map = match map.lock() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to lock data on a get_many request: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let files = map.inner.values().map(Clone::clone).collect::<Vec<_>>();
    Ok(Json(files))
}

async fn frontmatter_get_filename(
    State(markdown_files): State<frontmatter_file::map::ArcMutex>,
    Path(name): Path<String>,
) -> Result<(HeaderMap, String), StatusCode> {
    let map = markdown_files.0.as_ref();
    let map = match map.lock() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to lock data on a get_one request: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let file = map
        .inner
        .iter()
        .filter_map(|(path, file)| {
            let Some((os_filename, filename)) = path
                .file_name()
                .map(|os_filename| (os_filename, os_filename.to_str()))
            else {
                return None;
            };
            if let Some(f) = filename {
                Some((f, file))
            } else {
                eprintln!("Failed to parse a filename as UTF-8: {os_filename:?}");
                None
            }
        })
        .find(|(filename, _)| filename == &name);
    let Some((_, file)) = file else {
        return Err(StatusCode::NOT_FOUND);
    };

    let mut headers = HeaderMap::new();
    let frontmatter_string =
        serde_json::to_string(&file.frontmatter).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let frontmatter_header_value = frontmatter_string.parse().map_err(|err| {
        eprintln!("Failed to parse header value ({frontmatter_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-frontmatter", frontmatter_header_value);
    Ok((headers, file.body.clone()))
}

async fn run() -> Result<()> {
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

    let markdown_files = frontmatter_file::map::ArcMutex::new(markdown_files);

    // Automatically select the best implementation for your platform.
    let mut watcher = notify::recommended_watcher(markdown_files.clone())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&current_dir, RecursiveMode::NonRecursive)?;

    let app = Router::new()
        .route("/frontmatter/many", routing::get(frontmatter_get_many))
        .route(
            "/frontmatter/filename/:name",
            routing::get(frontmatter_get_filename),
        )
        .with_state(markdown_files);

    axum::Server::bind(&"0.0.0.0:4000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
