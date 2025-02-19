use serde::Serialize;

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

#[derive(Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
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

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use camino::Utf8PathBuf;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;
    use serde_yaml::Mapping;

    use crate::{
        frontmatter_file::{FrontmatterFile, Keeper},
        frontmatter_query::{FrontmatterQuery, QueryValue, Scalar},
    };

    macro_rules! s {
        ($v:literal) => {
            $v.to_string()
        };
    }

    macro_rules! path {
        ($v:literal) => {
            Utf8PathBuf::from_path_buf(PathBuf::from($v)).unwrap()
        };
    }

    macro_rules! dt {
        ($y:literal) => {
            chrono::Utc.with_ymd_and_hms($y, 0, 0, 0, 0, 0).unwrap()
        };
        ($y:literal, $m:literal) => {
            chrono::Utc.with_ymd_and_hms($y, $m, $d, 0, 0, 0).unwrap()
        };
        ($y:literal, $m:literal, $d:literal) => {
            chrono::Utc.with_ymd_and_hms($y, $m, $d, 0, 0, 0).unwrap()
        };
        ($y:literal, $m:literal, $d:literal, $h:literal) => {
            chrono::Utc.with_ymd_and_hms($y, $m, $d, $h, 0, 0).unwrap()
        };
        ($y:literal, $m:literal, $d:literal, $h:literal, $mm:literal) => {
            chrono::Utc
                .with_ymd_and_hms($y, $m, $d, $h, $mm, 0)
                .unwrap()
        };
    }

    fn make_test_keeper() -> Keeper {
        let mut hm = HashMap::new();
        let mut fm = Mapping::new();
        fm.insert(
            serde_yaml::Value::String(s!("tag")),
            serde_yaml::Value::String(s!("blue")),
        );
        hm.insert(
            path!("/something.md"),
            FrontmatterFile {
                name: s!("something.md"),
                frontmatter: None,
                body: s!(""),
                modified: dt!(2024, 1, 1, 6),
                created: dt!(2024, 1, 1, 5),
            },
        );
        hm.insert(
            path!("/about.md"),
            FrontmatterFile {
                name: s!("about.md"),
                frontmatter: Some(fm.clone()),
                body: s!(""),
                modified: dt!(2024, 1, 1, 11),
                created: dt!(2024, 1, 1, 9),
            },
        );
        hm.insert(
            path!("/blah.md"),
            FrontmatterFile {
                name: s!("blah.md"),
                frontmatter: Some(fm),
                body: s!(""),
                modified: dt!(2024, 1, 1, 16),
                created: dt!(2024, 1, 1, 15),
            },
        );
        Keeper { inner: hm }
    }

    #[test]
    fn get() {
        let keeper = make_test_keeper();

        let response = super::get(&keeper, "something.md", Some("created"), true).unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("about.md"), response.next_file_name);

        let response = super::get(&keeper, "about.md", Some("created"), true).unwrap();
        assert_eq!(Some("something.md"), response.prev_file_name);
        assert_eq!(Some("blah.md"), response.next_file_name);

        let response = super::get(&keeper, "blah.md", Some("created"), true).unwrap();
        assert_eq!(None, response.next_file_name);
        assert_eq!(Some("about.md"), response.prev_file_name);
    }

    #[test]
    fn query() {
        let keeper = make_test_keeper();

        let mut query_inner = HashMap::new();
        query_inner.insert(s!("tag"), QueryValue::Scalar(Scalar::String(s!("blue"))));
        let query = FrontmatterQuery(query_inner);

        let response =
            super::query(&keeper, "about.md", &query, Some("created"), true, false).unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("blah.md"), response.next_file_name);

        let response =
            super::query(&keeper, "blah.md", &query, Some("created"), true, false).unwrap();
        assert_eq!(Some("about.md"), response.prev_file_name);
        assert_eq!(None, response.next_file_name);

        let response = super::query(
            &keeper,
            "something.md",
            &query,
            Some("created"),
            true,
            false,
        )
        .unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("about.md"), response.next_file_name);
    }
}
