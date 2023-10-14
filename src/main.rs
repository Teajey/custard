mod frontmatter_file;
mod fs;

use std::collections::HashMap;

use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing, Json, Router};
use notify::{RecursiveMode, Watcher};

use frontmatter_file::FrontmatterFile;

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

async fn frontmatter_get_one(
    State(markdown_files): State<frontmatter_file::map::ArcMutex>,
) -> Result<Json<Option<FrontmatterFile>>, StatusCode> {
    let map = markdown_files.0.as_ref();
    let map = match map.lock() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to lock data on a get_one request: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let file = map.inner.values().next().map(Clone::clone);
    Ok(Json(file))
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
        .route("/frontmatter/one", routing::get(frontmatter_get_one))
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
