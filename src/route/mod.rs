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
) -> impl Iterator<Item = &'a FrontmatterFile> {
    files.filter(|file| {
        let Some(frontmatter) = file.frontmatter() else {
            // if query is '{}', include this
            return query.is_empty();
        };
        query.is_subset(&markup::yaml_to_json(frontmatter))
    })
}

fn lock_keeper(keeper: &keeper::ArcMutex) -> Result<MutexGuard<'_, Keeper>, StatusCode> {
    keeper.lock().map_err(|err| {
        eprintln!("Failed to lock files data: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
