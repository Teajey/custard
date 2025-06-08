use std::collections::HashMap;
use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use super::lock_keeper;

use custard_lib::{frontmatter_file, frontmatter_query::FrontmatterQueryMap};

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    Path(key): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = &*lock_keeper(&markdown_files)?;

    let values = custard_lib::collate::collate(
        keeper,
        custard_lib::collate::Args {
            key: key.as_str(),
            query: None,
        },
    );

    Ok(Json(values))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(key): Path<String>,
    Json(query_map): Json<FrontmatterQueryMap>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let keeper = &*lock_keeper(&markdown_files)?;

    let intersect = params
        .get("intersect")
        .map(|p| bool::from_str(p))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let values = custard_lib::collate::collate(
        keeper,
        custard_lib::collate::Args {
            key: key.as_str(),
            query: Some(custard_lib::frontmatter_query::FrontmatterQuery {
                map: query_map,
                intersect,
            }),
        },
    );

    Ok(Json(values))
}
