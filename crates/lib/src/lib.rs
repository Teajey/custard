#![allow(clippy::missing_errors_doc)]

pub mod collate;
pub mod frontmatter_file;
pub mod frontmatter_query;
mod fs;
pub mod list;
mod markup;
pub mod single;

use serde_yaml::Mapping;

use frontmatter_file::FrontmatterFile;
use frontmatter_query::FrontmatterQuery;

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
    query: FrontmatterQuery,
    name: Option<&'a str>,
) -> impl Iterator<Item = &'a FrontmatterFile> {
    files.filter(move |file| {
        if let Some(name) = name {
            if file.name == name {
                return true;
            }
        }
        let Some(frontmatter) = file.frontmatter() else {
            // if query is '{}', include this
            return query.map.is_empty();
        };
        if query.intersect {
            query.map.is_intersect(&markup::yaml_to_json(frontmatter))
        } else {
            query.map.is_subset(&markup::yaml_to_json(frontmatter))
        }
    })
}
