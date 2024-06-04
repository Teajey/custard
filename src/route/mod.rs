pub mod collate_strings;
pub mod frontmatter_file;
pub mod frontmatter_list;

use std::sync::MutexGuard;

use axum::http::StatusCode;

use crate::{
    frontmatter_file::{keeper, FrontmatterFile, Keeper},
    frontmatter_query::FrontmatterQuery,
    markup,
};

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

fn lock_keeper(keeper: &keeper::ArcMutex) -> Result<MutexGuard<'_, Keeper>, StatusCode> {
    keeper.lock().map_err(|err| {
        eprintln!("Failed to lock files data: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
