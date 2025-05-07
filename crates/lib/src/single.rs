use serde::{Deserialize, Serialize};
use tracing::debug;

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

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Get<'a> {
    pub name: &'a str,
    #[serde(default)]
    pub sort_key: Option<&'a str>,
    #[serde(default)]
    pub order_desc: bool,
}

impl<'a> Get<'a> {
    #[must_use]
    pub fn new(name: &'a str, sort_key: Option<&'a str>, order_desc: bool) -> Self {
        Self {
            name,
            sort_key,
            order_desc,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn inner_get<'a, 'b>(keeper: &'a Keeper, args: Get<'b>) -> Option<Response<'a>> {
    let mut files = keeper.files().collect::<Vec<&'a FrontmatterFile>>();

    sort_with_params(args.sort_key, args.order_desc, &mut files);

    let (i, file) = find_file_and_index(&files, args.name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&files, i);

    Some(Response {
        file,
        prev_file_name,
        next_file_name,
    })
}

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn get<'a>(keeper: &'a Keeper, args: Get<'_>) -> Option<Response<'a>> {
    debug!("Received get request: {args:?}");
    let response = inner_get(keeper, args);
    debug!("Sending get response: {response:?}");
    response
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Query<'a> {
    pub name: &'a str,
    pub query: FrontmatterQuery,
    #[serde(default)]
    pub sort_key: Option<&'a str>,
    #[serde(default)]
    pub order_desc: bool,
    #[serde(default)]
    pub intersect: bool,
}

impl<'a> Query<'a> {
    #[must_use]
    pub fn new(
        name: &'a str,
        query: FrontmatterQuery,
        sort_key: Option<&'a str>,
        order_desc: bool,
        intersect: bool,
    ) -> Self {
        Self {
            name,
            query,
            sort_key,
            order_desc,
            intersect,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn inner_query<'a, 'b: 'a>(keeper: &'a Keeper, args: Query<'b>) -> Option<Response<'a>> {
    let files = keeper.files();

    let mut filtered_files =
        query_files(files, args.query, Some(args.name), args.intersect).collect::<Vec<_>>();

    sort_with_params(args.sort_key, args.order_desc, &mut filtered_files);

    let (i, file) = find_file_and_index(&filtered_files, args.name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&filtered_files, i);

    Some(Response {
        file,
        prev_file_name,
        next_file_name,
    })
}

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn query<'a, 'b: 'a>(keeper: &'a Keeper, args: Query<'b>) -> Option<Response<'a>> {
    debug!("Received query request: {args:?}");
    let response = inner_query(keeper, args);
    debug!("Sending query response: {response:?}");
    response
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
            chrono::Utc.with_ymd_and_hms($y, $m, 0, 0, 0, 0).unwrap()
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

        let response = super::get(
            &keeper,
            super::Get::new("something.md", Some("created"), true),
        )
        .unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("about.md"), response.next_file_name);

        let response =
            super::get(&keeper, super::Get::new("about.md", Some("created"), true)).unwrap();
        assert_eq!(Some("something.md"), response.prev_file_name);
        assert_eq!(Some("blah.md"), response.next_file_name);

        let response =
            super::get(&keeper, super::Get::new("blah.md", Some("created"), true)).unwrap();
        assert_eq!(None, response.next_file_name);
        assert_eq!(Some("about.md"), response.prev_file_name);
    }

    #[test]
    fn query() {
        let keeper = make_test_keeper();

        let mut query_inner = HashMap::new();
        query_inner.insert(s!("tag"), QueryValue::Scalar(Scalar::String(s!("blue"))));
        let query = FrontmatterQuery(query_inner);

        let response = super::query(
            &keeper,
            super::Query::new("about.md", query.clone(), Some("created"), true, false),
        )
        .unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("blah.md"), response.next_file_name);

        let response = super::query(
            &keeper,
            super::Query::new("blah.md", query.clone(), Some("created"), true, false),
        )
        .unwrap();
        assert_eq!(Some("about.md"), response.prev_file_name);
        assert_eq!(None, response.next_file_name);

        let response = super::query(
            &keeper,
            super::Query::new("something.md", query, Some("created"), true, false),
        )
        .unwrap();
        assert_eq!(None, response.prev_file_name);
        assert_eq!(Some("about.md"), response.next_file_name);
    }
}
