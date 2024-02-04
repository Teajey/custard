use std::collections::HashMap;

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::get_sort_value;
use crate::{frontmatter_file, markup};
use crate::{frontmatter_file::FrontmatterFile, frontmatter_query::FrontmatterQuery};

fn assign_headers(file: &FrontmatterFile) -> Result<HeaderMap, StatusCode> {
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

    Ok(headers)
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = markdown_files.lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_file request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut files = keeper.files().collect::<Vec<_>>();

    let sort_key = params.get("sort");

    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_value = get_sort_value(f.frontmatter(), sort_key, f.created());
            let g_value = get_sort_value(g.frontmatter(), sort_key, g.created());
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }
    files.reverse();

    let (i, file) = files
        .iter()
        .enumerate()
        .filter(|(_, file)| {
            let Some(frontmatter) = file.frontmatter() else {
                // if query is '{}', include this
                return query.is_empty();
            };
            query.is_subset(&markup::yaml_to_json(frontmatter))
        })
        .find(|(_, file)| file.name() == name)
        .ok_or(StatusCode::NOT_FOUND)?;
    let prev_file_name = files.get(i - 1).map(|f| f.name());
    let next_file_name = files.get(i + 1).map(|f| f.name());

    let mut headers = assign_headers(file)?;

    if let Some(prev_file_name) = prev_file_name {
        let prev_file_name_header_value = prev_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'prev-file-name' header value ({prev_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-prev-file", prev_file_name_header_value);
    }

    if let Some(next_file_name) = next_file_name {
        let next_file_name_header_value = next_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'next-file-name' header value ({next_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-next-file", next_file_name_header_value);
    }

    Ok((headers, file.body().to_owned()))
}

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(name): Path<String>,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = markdown_files.lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_file request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let file = keeper
        .files()
        .find(|file| file.name() == name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let headers = assign_headers(file)?;

    Ok((headers, file.body().to_owned()))
}
