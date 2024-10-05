use std::collections::HashMap;
use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use super::lock_keeper;

use custard_lib::{frontmatter_file, frontmatter_query::FrontmatterQuery};

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(key): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = &*lock_keeper(&markdown_files)?;

    let values = custard_lib::collate::get(keeper, &key);

    Ok(Json(values))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(key): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = &*lock_keeper(&markdown_files)?;

    let intersect = params
        .get("intersect")
        .map(|p| bool::from_str(p))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let values = custard_lib::collate::query(keeper, &key, &query, intersect);

    Ok(Json(values))
}
