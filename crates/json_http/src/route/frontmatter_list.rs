use std::{collections::HashMap, ops::Deref};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use custard_lib::{frontmatter_file, frontmatter_query::FrontmatterQuery};

use super::lock_keeper;

fn assign_headers(file_count: usize) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert("x-length", file_count.into());

    headers
}

fn get_inner(
    params: &HashMap<String, String>,
    files: &frontmatter_file::keeper::ArcMutex,
) -> Result<(HeaderMap, Vec<frontmatter_file::Short>), StatusCode> {
    let keeper = &*lock_keeper(files)?;

    let sort_key = params.get("sort").map(Deref::deref);
    let order_desc = "desc" == params.get("order").map_or("desc", Deref::deref);
    let offset = params
        .get("offset")
        .map(|x| x.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = params
        .get("limit")
        .map(|x| x.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let files = custard_lib::list::get(keeper, sort_key, order_desc, offset, limit);

    let headers = assign_headers(files.len());

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
    let keeper = &*lock_keeper(files)?;

    let sort_key = params.get("sort").map(Deref::deref);
    let order_desc = "desc" == params.get("order").map_or("desc", Deref::deref);
    let offset = params
        .get("offset")
        .map(|x| x.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = params
        .get("limit")
        .map(|x| x.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let intersect = params
        .get("intersect")
        .map(|p| p.parse())
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let files = custard_lib::list::query(
        keeper, query, sort_key, order_desc, offset, limit, intersect,
    );

    let headers = assign_headers(files.len());

    Ok((headers, files))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, Json<Vec<frontmatter_file::Short>>), StatusCode> {
    let (headers, files) = post_inner(&params, &markdown_files, &query)?;

    Ok((headers, Json(files)))
}
