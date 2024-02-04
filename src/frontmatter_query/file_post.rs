use std::collections::HashMap;

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{frontmatter_file, markup};
use crate::{frontmatter_file::Short, frontmatter_query::FrontmatterQuery};

use super::get_sort_value;

pub async fn run(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = markdown_files.0.as_ref().lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_file request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut files = keeper.files().collect::<Vec<_>>();

    let sort_key = params.get("sort");

    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_short = Short::from((*f).clone());
            let g_short = Short::from((*g).clone());
            let f_value = get_sort_value(&f_short, sort_key);
            let g_value = get_sort_value(&g_short, sort_key);
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
    let prev_file = files.get(i - 1);
    let next_file = files.get(i + 1);

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

    if let Some(prev_file) = prev_file {
        let prev_file_name = prev_file.name();
        let prev_file_name_header_value = prev_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'prev-file-name' header value ({prev_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-prev-file", prev_file_name_header_value);
    }

    if let Some(next_file) = next_file {
        let next_file_name = next_file.name();
        let next_file_name_header_value = next_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'next-file-name' header value ({next_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-next-file", next_file_name_header_value);
    }

    Ok((headers, file.body().to_owned()))
}
