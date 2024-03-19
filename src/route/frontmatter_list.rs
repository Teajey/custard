use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    frontmatter_file::{self, Short},
    frontmatter_query::FrontmatterQuery,
};

use super::{lock_keeper, query_files};

fn assign_headers(file_count: usize) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert("x-length", file_count.into());

    headers
}

fn get_sort_value(file: &Short, sort_key: &str) -> String {
    file.frontmatter
        .as_ref()
        .and_then(|m| m.get(sort_key))
        .map(serde_yaml::to_string)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            serde_yaml::to_string(&file.created).expect("DateTime<Utc> must serialize")
        })
}

fn sort_with_params(params: &HashMap<String, String>, files: &mut [Short]) {
    let sort_key = params.get("sort");

    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_value = get_sort_value(f, sort_key);
            let g_value = get_sort_value(g, sort_key);
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }
    files.reverse();
}

fn paginate(params: &HashMap<String, String>, files: Vec<Short>) -> Result<Vec<Short>, StatusCode> {
    let offset = params
        .get("offset")
        .map(|x| x.parse::<usize>())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = params
        .get("limit")
        .map(|x| x.parse::<usize>())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let files = match (offset, limit) {
        (None, None) => files,
        (None, Some(limit)) => files.into_iter().take(limit).collect(),
        (Some(offset), None) => files.into_iter().skip(offset).collect(),
        (Some(offset), Some(limit)) => files.into_iter().skip(offset).take(limit).collect(),
    };
    Ok(files)
}

fn get_inner(
    params: &HashMap<String, String>,
    files: &frontmatter_file::keeper::ArcMutex,
) -> Result<(HeaderMap, Vec<frontmatter_file::Short>), StatusCode> {
    let keeper = lock_keeper(files)?;

    let mut files = keeper.files().cloned().map(Short::from).collect::<Vec<_>>();

    sort_with_params(params, &mut files);

    let headers = assign_headers(files.len());

    let files = paginate(params, files)?;

    Ok((headers, files))
}

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
) -> Result<(HeaderMap, Json<Vec<frontmatter_file::Short>>), StatusCode> {
    let (headers, files) = get_inner(&params, &markdown_files)?;

    Ok((headers, Json(files)))
}

fn post_inner(
    params: &HashMap<String, String>,
    files: &frontmatter_file::keeper::ArcMutex,
    query: &FrontmatterQuery,
) -> Result<(HeaderMap, Vec<frontmatter_file::Short>), StatusCode> {
    let keeper = lock_keeper(files)?;

    let files = keeper.files();

    let mut filtered_files = query_files(files, query, None)
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();

    sort_with_params(params, &mut filtered_files);

    let headers = assign_headers(filtered_files.len());

    let filtered_files = paginate(params, filtered_files)?;

    Ok((headers, filtered_files))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, Json<Vec<frontmatter_file::Short>>), StatusCode> {
    let (headers, files) = post_inner(&params, &markdown_files, &query)?;

    Ok((headers, Json(files)))
}
