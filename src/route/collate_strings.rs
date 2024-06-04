use std::collections::HashMap;
use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use super::{lock_keeper, query_files};

use crate::{
    frontmatter_file::{self, FrontmatterFile},
    frontmatter_query::FrontmatterQuery,
};

fn collate_strings_from_files<'a>(
    files: impl Iterator<Item = &'a FrontmatterFile>,
    key: &str,
) -> Vec<String> {
    files
        .filter_map(|fmf| fmf.frontmatter())
        .filter_map(|fm| fm.get(key))
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
        .collect()
}

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(key): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = lock_keeper(&markdown_files)?;
    let files = keeper.files();

    let mut values = collate_strings_from_files(files, &key);

    values.sort();
    values.dedup();

    Ok(Json(values))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(key): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = lock_keeper(&markdown_files)?;
    let files = keeper.files();

    let intersect = params
        .get("intersect")
        .map(|p| bool::from_str(p))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let files = query_files(files, &query, None, intersect);

    let mut values = collate_strings_from_files(files, &key);

    values.sort();
    values.dedup();

    Ok(Json(values))
}
