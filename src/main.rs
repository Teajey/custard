mod frontmatter_file;
mod frontmatter_query;
mod fs;
mod utf8_filepath;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing, Json, Router,
};
use frontmatter_query::FrontmatterQuery;
use notify::{RecursiveMode, Watcher};

use frontmatter_file::FrontmatterFile;
use utf8_filepath::UTF8FilePath;

async fn frontmatter_get_many(
    State(markdown_files): State<frontmatter_file::map::ArcMutex>,
) -> Result<Json<Vec<frontmatter_file::Named>>, StatusCode> {
    let map = markdown_files.0.as_ref();
    let map = match map.lock() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to lock data on a get_many request: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let files = map
        .inner
        .iter()
        .map(|(path, file)| frontmatter_file::Named::from_map_entry((path, file)))
        .collect();
    Ok(Json(files))
}

async fn frontmatter_post_many(
    State(markdown_files): State<frontmatter_file::map::ArcMutex>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<Json<Vec<frontmatter_file::Named>>, StatusCode> {
    let map = markdown_files.0.as_ref();
    let map = match map.lock() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Failed to lock data on a get_many request: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let files = map
        .inner
        .iter()
        .filter(|(_, file)| {
            let Some(frontmatter) = &file.frontmatter else {
                return query.is_empty();
            };
            query.is_subset(
                &serde_json::from_value(
                    serde_json::to_value(frontmatter).expect("valid yaml must map to valid json"),
                )
                .expect("Map<String, Value> is valid json"),
            )
        })
        .map(|(path, file)| frontmatter_file::Named::from_map_entry((path, file)))
        .collect();
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
        .find(|(filepath, _)| filepath.name() == name);
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
    let mut args = std::env::args();
    let port = args
        .nth(1)
        .ok_or_else(|| anyhow!("Expected a port number as a first argument"))?;
    if let Some(wd) = args.next() {
        std::env::set_current_dir(wd)?;
    }

    let current_dir = std::env::current_dir()?;
    let markdown_fps = fs::filepaths_with_extensions(&current_dir, &["md"])?
        .into_iter()
        .map(UTF8FilePath::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| anyhow!("Target paths are not all UTF-8 files: {err}"))?;
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
        .route("/frontmatter/many", routing::post(frontmatter_post_many))
        .route("/frontmatter/many", routing::get(frontmatter_get_many))
        .route(
            "/frontmatter/filename/:name",
            routing::get(frontmatter_get_filename),
        )
        .with_state(markdown_files);

    let socket_addr_string = format!("0.0.0.0:{port}");
    println!("Binding to {socket_addr_string}");
    axum::Server::bind(&socket_addr_string.parse()?)
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
