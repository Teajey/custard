pub mod collate_strings;
pub mod frontmatter_file;
pub mod frontmatter_list;

use std::sync::MutexGuard;

use axum::http::StatusCode;
use custard_lib::frontmatter_file::{keeper, Keeper};

fn lock_keeper(keeper: &keeper::ArcMutex) -> Result<MutexGuard<'_, Keeper>, StatusCode> {
    keeper.lock().map_err(|err| {
        eprintln!("Failed to lock files data: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
