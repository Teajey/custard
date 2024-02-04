use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

use crate::frontmatter_file;

pub async fn run(
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
