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

#[derive(Debug, Deserialize)]
pub struct Args<'a> {
    pub key: &'a str,
    #[serde(default)]
    pub query: Option<FrontmatterQuery>,
}

fn inner(keeper: &Keeper, args: Args<'_>) -> Vec<String> {
    let files = keeper.files();

    let mut values = if let Some(query) = args.query {
        let files = query_files(files, query, None);
        collate_strings_from_files(files, args.key)
    } else {
        collate_strings_from_files(files, args.key)
    };

    values.sort();
    values.dedup();

    values
}

#[must_use]
pub fn collate(keeper: &Keeper, args: Args<'_>) -> Vec<String> {
    debug!("Received collate request: {args:?}");
    let response = inner(keeper, args);
    debug!("Sending collate response: {response:?}");
    response
}
