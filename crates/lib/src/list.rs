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

pub fn get(
    keeper: &Keeper,
    sort_key: Option<&str>,
    order_desc: bool,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vec<frontmatter_file::Short> {
    let mut files = keeper.files().cloned().map(Short::from).collect::<Vec<_>>();

    sort_with_params(sort_key, order_desc, &mut files);

    paginate(files, offset, limit)
}

#[must_use]
pub fn query(
    keeper: &Keeper,
    query: FrontmatterQuery,
    sort_key: Option<&str>,
    order_desc: bool,
    offset: Option<usize>,
    limit: Option<usize>,
    intersect: bool,
) -> Vec<frontmatter_file::Short> {
    let files = keeper.files();

    let mut filtered_files = query_files(files, query, None, intersect)
        .map(|file| file.clone().into())
        .collect::<Vec<_>>();

    sort_with_params(sort_key, order_desc, &mut filtered_files);

    paginate(filtered_files, offset, limit)
}
