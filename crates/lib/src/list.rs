use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::frontmatter_file::{self, Keeper, Short};
use crate::frontmatter_query::{FrontmatterQuery, FrontmatterQueryMap};
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

#[derive(Debug, Deserialize)]
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
fn inner_get(keeper: &Keeper, args: Get<'_>) -> Response {
    let mut files = keeper.files().cloned().map(Short::from).collect::<Vec<_>>();

    let total = files.len();

    sort_with_params(args.sort_key, args.order_desc, &mut files);

    let files = paginate(files, args.offset, args.limit);

    Response { files, total }
}

#[allow(clippy::needless_pass_by_value)]
pub fn get(keeper: &Keeper, args: Get<'_>) -> Response {
    debug!("Received get request: {args:?}");
    let response = inner_get(keeper, args);
    debug!("Sending get response: {response:?}");
    response
}

#[derive(Debug, Deserialize)]
pub struct Args<'a> {
    #[serde(default)]
    pub query: Option<FrontmatterQuery>,
    #[serde(default)]
    pub sort_key: Option<&'a str>,
    #[serde(default)]
    pub order_desc: bool,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl<'a> Args<'a> {
    #[must_use]
    pub fn get(
        sort_key: Option<&'a str>,
        order_desc: bool,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Self {
        Self {
            query: None,
            sort_key,
            order_desc,
            offset,
            limit,
        }
    }

    #[must_use]
    pub fn query(
        map: FrontmatterQueryMap,
        sort_key: Option<&'a str>,
        order_desc: bool,
        offset: Option<usize>,
        limit: Option<usize>,
        intersect: bool,
    ) -> Self {
        Self {
            query: Some(FrontmatterQuery { map, intersect }),
            sort_key,
            order_desc,
            offset,
            limit,
        }
    }
}

fn inner_query(keeper: &Keeper, args: Args<'_>) -> Response {
    let files = keeper.files();

    let mut files = if let Some(query) = args.query {
        query_files(files, query, None)
            .cloned()
            .map(Short::from)
            .collect::<Vec<_>>()
    } else {
        files.cloned().map(Short::from).collect::<Vec<_>>()
    };

    let total = files.len();

    sort_with_params(args.sort_key, args.order_desc, &mut files);

    let files = paginate(files, args.offset, args.limit);

    Response { files, total }
}

#[must_use]
pub fn query(keeper: &Keeper, args: Args<'_>) -> Response {
    debug!("Received query request: {args:?}");
    let response = inner_query(keeper, args);
    debug!("Sending query response: {response:?}");
    response
}
