use crate::frontmatter_file::{FrontmatterFile, Keeper};
use crate::frontmatter_query::FrontmatterQuery;
use crate::{get_sort_value, query_files};

fn sort_with_params(sort_key: Option<&str>, order_desc: bool, files: &mut [&FrontmatterFile]) {
    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_value = get_sort_value(f.frontmatter(), &f.created, sort_key);
            let g_value = get_sort_value(g.frontmatter(), &g.created, sort_key);
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }

    if order_desc {
        files.reverse();
    }
}

fn find_file_and_index<'a>(
    files: &[&'a FrontmatterFile],
    name: &str,
) -> Option<(usize, &'a FrontmatterFile)> {
    files
        .iter()
        .enumerate()
        .find(|(_, file)| file.name() == name)
        .map(|(i, file)| (i, *file))
}

fn get_prev_and_next_file_names<'a>(
    files: &[&'a FrontmatterFile],
    i: usize,
) -> (Option<&'a str>, Option<&'a str>) {
    let prev_file_name = if i > 0 {
        Some(files[i - 1].name())
    } else {
        None
    };
    let next_file_name = files.get(i + 1).map(|f| f.name());
    (next_file_name, prev_file_name)
}

pub struct Response<'a> {
    pub file: &'a FrontmatterFile,
    pub prev_file_name: Option<&'a str>,
    pub next_file_name: Option<&'a str>,
}

#[must_use]
pub fn get<'a>(
    keeper: &'a Keeper,
    name: &str,
    sort_key: Option<&str>,
    order_desc: bool,
) -> Option<Response<'a>> {
    let mut files = keeper.files().collect::<Vec<&'a FrontmatterFile>>();

    sort_with_params(sort_key, order_desc, &mut files);

    let (i, file) = find_file_and_index(&files, name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&files, i);

    Some(Response {
        file,
        prev_file_name,
        next_file_name,
    })
}

#[must_use]
pub fn query<'a, 'b: 'a>(
    keeper: &'a Keeper,
    name: &'b str,
    query: &'b FrontmatterQuery,
    sort_key: Option<&str>,
    order_desc: bool,
    intersect: bool,
) -> Option<Response<'a>> {
    let files = keeper.files();

    let mut filtered_files = query_files(files, query, Some(name), intersect).collect::<Vec<_>>();

    sort_with_params(sort_key, order_desc, &mut filtered_files);

    let (i, file) = find_file_and_index(&filtered_files, name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&filtered_files, i);

    Some(Response {
        file,
        prev_file_name,
        next_file_name,
    })
}
