use serde::{Deserialize, Serialize};

use crate::frontmatter_file::{self, Keeper, Short};
use crate::frontmatter_query::FrontmatterQuery;
use crate::{get_sort_value, query_files};

fn sort_with_params(sort_key: Option<&str>, order_desc: bool, files: &mut [Short]) {
    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_value = get_sort_value(f.frontmatter.as_ref(), &f.created, sort_key);
            let g_value = get_sort_value(g.frontmatter.as_ref(), &g.created, sort_key);
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }

    if order_desc {
        files.reverse();
    }
}

fn paginate(files: Vec<Short>, offset: Option<usize>, limit: Option<usize>) -> Vec<Short> {
    match (offset, limit) {
        (None, None) => files,
        (None, Some(limit)) => files.into_iter().take(limit).collect(),
        (Some(offset), None) => files.into_iter().skip(offset).collect(),
        (Some(offset), Some(limit)) => files.into_iter().skip(offset).take(limit).collect(),
    }
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Get<'a> {
    #[serde(default)]
    pub sort_key: Option<&'a str>,
    #[serde(default)]
    pub order_desc: bool,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl<'a> Get<'a> {
    #[must_use]
    pub fn new(
        sort_key: Option<&'a str>,
        order_desc: bool,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Self {
        Self {
            sort_key,
            order_desc,
            offset,
            limit,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub files: Vec<frontmatter_file::Short>,
    pub total: usize,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get(keeper: &Keeper, args: Get<'_>) -> Response {
    let mut files = keeper.files().cloned().map(Short::from).collect::<Vec<_>>();

    let total = files.len();

    sort_with_params(args.sort_key, args.order_desc, &mut files);

    let files = paginate(files, args.offset, args.limit);

    Response { files, total }
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Query<'a> {
    pub query: FrontmatterQuery,
    #[serde(default)]
    pub sort_key: Option<&'a str>,
    #[serde(default)]
    pub order_desc: bool,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub intersect: bool,
}

impl<'a> Query<'a> {
    #[must_use]
    pub fn new(
        query: FrontmatterQuery,
        sort_key: Option<&'a str>,
        order_desc: bool,
        offset: Option<usize>,
        limit: Option<usize>,
        intersect: bool,
    ) -> Self {
        Self {
            query,
            sort_key,
            order_desc,
            offset,
            limit,
            intersect,
        }
    }
}

#[must_use]
pub fn query(keeper: &Keeper, args: Query<'_>) -> Response {
    let files = keeper.files();

    let mut filtered_files = query_files(files, args.query, None, args.intersect)
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();

    let total = filtered_files.len();

    sort_with_params(args.sort_key, args.order_desc, &mut filtered_files);

    let files = paginate(filtered_files, args.offset, args.limit);

    Response { files, total }
}
