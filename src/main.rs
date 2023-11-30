mod frontmatter_file;
mod frontmatter_query;
mod fs;

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing, Json, Router,
};
use camino::Utf8PathBuf;
use frontmatter_query::FrontmatterQuery;
use notify::{RecursiveMode, Watcher};

async fn frontmatter_query_post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<Json<Vec<frontmatter_file::Short>>, StatusCode> {
    let keeper = markdown_files.0.as_ref().lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_many request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut files = keeper
        .files()
        .filter(|file| {
            let Some(frontmatter) = file.frontmatter() else {
                return query.is_empty();
            };
            query.is_subset(
                &serde_json::from_value(
                    serde_json::to_value(frontmatter).expect("valid yaml must map to valid json"),
                )
                .expect("Map<String, Value> is valid json"),
            )
        })
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();
    files.sort();
    files.reverse();
    Ok(Json(files))
}

async fn frontmatter_list_get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
) -> Result<Json<Vec<frontmatter_file::Short>>, StatusCode> {
    let keeper = markdown_files.0.as_ref().lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_many request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut files = keeper
        .files()
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();
    files.sort();
    files.reverse();
    Ok(Json(files))
}

async fn frontmatter_file_get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(name): Path<String>,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = markdown_files.0.as_ref().lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_file request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let file = keeper
        .files()
        .find(|file| file.name() == name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut headers = HeaderMap::new();
    let frontmatter = file.frontmatter();
    let frontmatter_string = serde_json::to_string(&frontmatter).map_err(|err| {
        eprintln!(
            "Failed to serialize frontmatter ({frontmatter:?}) as JSON during get request: {err}"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let frontmatter_header_value = frontmatter_string.parse().map_err(|err| {
        eprintln!("Failed to parse header value ({frontmatter_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-frontmatter", frontmatter_header_value);

    let created_string = file.created().to_rfc3339();
    let created_header_value = created_string.parse().map_err(|err| {
        eprintln!("Failed to parse 'created' header value ({created_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-created", created_header_value);

    let modified_string = file.modified().to_rfc3339();
    let modified_header_value = modified_string.parse().map_err(|err| {
        eprintln!("Failed to parse 'modified' header value ({modified_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-modified", modified_header_value);

    Ok((headers, file.body().to_owned()))
}

async fn frontmatter_collate_strings_get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(key): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = markdown_files.0.as_ref().lock().map_err(|err| {
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
        .route("/frontmatter/query", routing::post(frontmatter_query_post))
        .route("/frontmatter/list", routing::get(frontmatter_list_get))
        .route(
            "/frontmatter/file/:name",
            routing::get(frontmatter_file_get),
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
