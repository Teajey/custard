use serde::Deserialize;
use tracing::debug;

use super::query_files;

use crate::{
    frontmatter_file::{FrontmatterFile, Keeper},
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

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Get<'a> {
    pub key: &'a str,
}

impl<'a> Get<'a> {
    #[must_use]
    pub fn new(key: &'a str) -> Self {
        Self { key }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn inner_get(keeper: &Keeper, args: Get<'_>) -> Vec<String> {
    let files = keeper.files();

    let mut values = collate_strings_from_files(files, args.key);

    values.sort();
    values.dedup();

    values
}

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn get(keeper: &Keeper, args: Get<'_>) -> Vec<String> {
    debug!("Received get request: {args:?}");
    let response = inner_get(keeper, args);
    debug!("Sending get response: {response:?}");
    response
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Query<'a> {
    pub key: &'a str,
    pub query: FrontmatterQuery,
    #[serde(default)]
    pub intersect: bool,
}

impl<'a> Query<'a> {
    #[must_use]
    pub fn new(key: &'a str, query: FrontmatterQuery, intersect: bool) -> Self {
        Self {
            key,
            query,
            intersect,
        }
    }
}

fn inner_query(keeper: &Keeper, args: Query<'_>) -> Vec<String> {
    let files = keeper.files();

    let files = query_files(files, args.query, None, args.intersect);

    let mut values = collate_strings_from_files(files, args.key);

    values.sort();
    values.dedup();

    values
}

#[must_use]
pub fn query(keeper: &Keeper, args: Query<'_>) -> Vec<String> {
    debug!("Received query request: {args:?}");
    let response = inner_query(keeper, args);
    debug!("Sending query response: {response:?}");
    response
}
