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

#[must_use]
pub fn get(keeper: &Keeper, key: &str) -> Vec<String> {
    let files = keeper.files();

    let mut values = collate_strings_from_files(files, key);

    values.sort();
    values.dedup();

    values
}

#[must_use]
pub fn query(keeper: &Keeper, key: &str, query: &FrontmatterQuery, intersect: bool) -> Vec<String> {
    let files = keeper.files();

    let files = query_files(files, query, None, intersect);

    let mut values = collate_strings_from_files(files, key);

    values.sort();
    values.dedup();

    values
}
