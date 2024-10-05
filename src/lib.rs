#![allow(clippy::missing_errors_doc)]

mod frontmatter_file;
mod frontmatter_query;
mod fs;
pub mod list;
mod markup;
pub mod single;

use frontmatter_file::FrontmatterFile;
use frontmatter_query::FrontmatterQuery;
use serde_yaml::Mapping;

fn get_sort_value(
    frontmatter: Option<&Mapping>,
    created: &chrono::DateTime<chrono::Utc>,
    sort_key: &str,
) -> String {
    frontmatter
        .and_then(|m| m.get(sort_key))
        .map(serde_yaml::to_string)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_else(|| serde_yaml::to_string(created).expect("DateTime<Utc> must serialize"))
}

fn query_files<'a>(
    files: impl Iterator<Item = &'a FrontmatterFile>,
    query: &'a FrontmatterQuery,
    name: Option<&'a str>,
    intersect: bool,
) -> impl Iterator<Item = &'a FrontmatterFile> {
    files.filter(move |file| {
        if let Some(name) = name {
            if file.name == name {
                return true;
            }
        }
        let Some(frontmatter) = file.frontmatter() else {
            // if query is '{}', include this
            return query.is_empty();
        };
        if intersect {
            query.is_intersect(&markup::yaml_to_json(frontmatter))
        } else {
            query.is_subset(&markup::yaml_to_json(frontmatter))
        }
    })
}
