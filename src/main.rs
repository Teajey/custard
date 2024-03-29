mod frontmatter_file;
mod frontmatter_query;
mod fs;
mod markup;
mod route;

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing, Json, Router,
};
use camino::Utf8PathBuf;
use notify::{RecursiveMode, Watcher};

async fn frontmatter_collate_strings_get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(key): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = markdown_files.lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_collate_strings request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut values = keeper
        .files()
        .filter_map(|fmf| fmf.frontmatter())
        .filter_map(|fm| fm.get(&key))
        .filter_map(|v| match v {
            serde_yaml::Value::String(v) => Some(vec![v.clone()]),
            serde_yaml::Value::Sequence(seq) => seq
                .iter()
                .map(|v| match v {
                    serde_yaml::Value::String(v) => Some(v.clone()),
                    _ => None,
                })
                .collect(),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    Ok(Json(values))
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
    let current_dir = Utf8PathBuf::try_from(current_dir)?;

    let keeper = frontmatter_file::Keeper::new(&current_dir)?;

    let markdown_files = frontmatter_file::keeper::ArcMutex::new(keeper);

    let mut watcher = notify::recommended_watcher(markdown_files.clone())?;

    watcher.watch(current_dir.as_std_path(), RecursiveMode::NonRecursive)?;

    let app = Router::new()
        .route(
            "/frontmatter/list",
            routing::post(route::frontmatter_list::post).get(route::frontmatter_list::get),
        )
        .route(
            "/frontmatter/file/:name",
            routing::post(route::frontmatter_file::post).get(route::frontmatter_file::get),
        )
        .route(
            "/frontmatter/collate_strings/:key",
            routing::get(frontmatter_collate_strings_get),
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
