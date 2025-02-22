use std::{collections::HashMap, ops::Deref};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use custard_lib::{
    frontmatter_file::{self, FrontmatterFile},
    frontmatter_query::FrontmatterQuery,
};

use super::lock_keeper;

fn assign_headers(
    file: &FrontmatterFile,
    prev_file_name: Option<&str>,
    next_file_name: Option<&str>,
) -> Result<HeaderMap, StatusCode> {
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

    Ok(headers)
}

fn post_inner(
    files: &frontmatter_file::keeper::ArcMutex,
    params: &HashMap<String, String>,
    name: &str,
    query: FrontmatterQuery,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = &*lock_keeper(files)?;

    let order_desc = "desc" == params.get("order").map_or("desc", Deref::deref);
    let sort_key = params.get("sort").map(Deref::deref);
    let intersect = params
        .get("intersect")
        .map(|p| p.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let response = custard_lib::single::query(
        keeper,
        custard_lib::single::Query::new(name, query, sort_key, order_desc, intersect),
    )
    .ok_or(StatusCode::NOT_FOUND)?;

    let headers = assign_headers(
        response.file,
        response.prev_file_name,
        response.next_file_name,
    )?;

    Ok((headers, response.file.body().to_owned()))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, String), StatusCode> {
    let result = post_inner(&markdown_files, &params, &name, query)?;

    Ok(result)
}

fn get_inner(
    files: &frontmatter_file::keeper::ArcMutex,
    params: &HashMap<String, String>,
    name: &str,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = &*lock_keeper(files)?;

    let order_desc = "desc" == params.get("order").map_or("desc", Deref::deref);
    let sort_key = params.get("sort").map(Deref::deref);

    let response = custard_lib::single::get(
        keeper,
        custard_lib::single::Get::new(name, sort_key, order_desc),
    )
    .ok_or(StatusCode::NOT_FOUND)?;

    let headers = assign_headers(
        response.file,
        response.prev_file_name,
        response.next_file_name,
    )?;

    Ok((headers, response.file.body().to_owned()))
}

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
) -> Result<(HeaderMap, String), StatusCode> {
    let result = get_inner(&markdown_files, &params, &name)?;

    Ok(result)
}
