use std::collections::HashMap;

use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    frontmatter_file::{self, Short},
    frontmatter_query::FrontmatterQuery,
    get_sort_value, markup,
};

pub async fn get(
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

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<Json<Vec<frontmatter_file::Short>>, StatusCode> {
    let keeper = markdown_files.lock().map_err(|err| {
        eprintln!("Failed to lock data on a get_many request: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut files = keeper
        .files()
        .filter(|file| {
            let Some(frontmatter) = file.frontmatter() else {
                // if query is '{}', include this
                return query.is_empty();
            };
            query.is_subset(&markup::yaml_to_json(frontmatter))
        })
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();

    let sort_key = params.get("sort");

    if let Some(sort_key) = sort_key {
        files.sort_by(|f: &Short, g| {
            let f_value = get_sort_value(f.frontmatter.as_ref(), sort_key, &f.created);
            let g_value = get_sort_value(g.frontmatter.as_ref(), sort_key, &g.created);
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }
    files.reverse();
    Ok(Json(files))
}
