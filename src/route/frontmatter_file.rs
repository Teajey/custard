use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::frontmatter_file;
use crate::{frontmatter_file::FrontmatterFile, frontmatter_query::FrontmatterQuery};

use super::{lock_keeper, query_files};

fn get_sort_value(file: &FrontmatterFile, sort_key: &str) -> String {
    file.frontmatter()
        .and_then(|m| m.get(sort_key))
        .map(serde_yaml::to_string)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            serde_yaml::to_string(&file.created).expect("DateTime<Utc> must serialize")
        })
}

fn sort_with_params(params: &HashMap<String, String>, files: &mut [&FrontmatterFile]) {
    let sort_key = params.get("sort");
    let order_key = params.get("order").map_or("desc", String::as_str);

    if let Some(sort_key) = sort_key {
        files.sort_by(|f, g| {
            let f_value = get_sort_value(f, sort_key);
            let g_value = get_sort_value(g, sort_key);
            f_value.cmp(&g_value)
        });
    } else {
        files.sort();
    }

    if order_key == "desc" {
        files.reverse();
    }
}

fn assign_headers(
    file: &FrontmatterFile,
    prev_file_name: Option<&str>,
    next_file_name: Option<&str>,
) -> Result<HeaderMap, StatusCode> {
    let mut headers = HeaderMap::new();
    let frontmatter = file.frontmatter();
    let frontmatter_string = serde_json::to_string(&frontmatter).map_err(|err| {
        eprintln!(
            "Failed to serialize frontmatter ({frontmatter:?}) as JSON during get request: {err}"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let frontmatter_header_value = frontmatter_string.parse().map_err(|err| {
        eprintln!("Failed to parse header value ({frontmatter_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-frontmatter", frontmatter_header_value);

    let created_string = file.created().to_rfc3339();
    let created_header_value = created_string.parse().map_err(|err| {
        eprintln!("Failed to parse 'created' header value ({created_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-created", created_header_value);

    let modified_string = file.modified().to_rfc3339();
    let modified_header_value = modified_string.parse().map_err(|err| {
        eprintln!("Failed to parse 'modified' header value ({modified_string:?}): {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    headers.insert("x-modified", modified_header_value);

    if let Some(prev_file_name) = prev_file_name {
        let prev_file_name_header_value = prev_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'prev-file-name' header value ({prev_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-prev-file", prev_file_name_header_value);
    }

    if let Some(next_file_name) = next_file_name {
        let next_file_name_header_value = next_file_name.parse().map_err(|err| {
            eprintln!("Failed to parse 'next-file-name' header value ({next_file_name:?}): {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        headers.insert("x-next-file", next_file_name_header_value);
    }

    Ok(headers)
}

fn find_file_and_index<'a>(
    files: &'a [&'a FrontmatterFile],
    name: &str,
) -> Result<(usize, &'a FrontmatterFile), StatusCode> {
    files
        .iter()
        .enumerate()
        .find(|(_, file)| file.name() == name)
        .map(|(i, file)| (i, *file))
        .ok_or(StatusCode::NOT_FOUND)
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
    (prev_file_name, next_file_name)
}

fn post_inner(
    files: &frontmatter_file::keeper::ArcMutex,
    params: &HashMap<String, String>,
    name: &str,
    query: &FrontmatterQuery,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = lock_keeper(files)?;

    let files = keeper.files().collect::<Vec<_>>();

    let intersect = params
        .get("intersect")
        .map(|p| bool::from_str(p))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or_default();

    let mut filtered_files =
        query_files(files.clone().into_iter(), query, Some(name), intersect).collect::<Vec<_>>();

    sort_with_params(params, &mut filtered_files);

    let (i, file) = find_file_and_index(&filtered_files, name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&filtered_files, i);

    let headers = assign_headers(file, prev_file_name, next_file_name)?;

    Ok((headers, file.body().to_owned()))
}

pub async fn post(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
    Json(query): Json<FrontmatterQuery>,
) -> Result<(HeaderMap, String), StatusCode> {
    let result = post_inner(&markdown_files, &params, &name, &query)?;

    Ok(result)
}

fn get_inner(
    files: &frontmatter_file::keeper::ArcMutex,
    params: &HashMap<String, String>,
    name: &str,
) -> Result<(HeaderMap, String), StatusCode> {
    let keeper = lock_keeper(files)?;

    let mut files = keeper.files().collect::<Vec<_>>();

    sort_with_params(params, &mut files);

    let (i, file) = find_file_and_index(&files, name)?;

    let (prev_file_name, next_file_name) = get_prev_and_next_file_names(&files, i);

    let headers = assign_headers(file, prev_file_name, next_file_name)?;

    Ok((headers, file.body().to_owned()))
}

pub async fn get(
    State(markdown_files): State<frontmatter_file::keeper::ArcMutex>,
    params: Query<HashMap<String, String>>,
    Path(name): Path<String>,
) -> Result<(HeaderMap, String), StatusCode> {
    let result = get_inner(&markdown_files, &params, &name)?;

    Ok(result)
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use camino::Utf8PathBuf;
    use chrono::TimeZone;
    use serde_yaml::Mapping;

    use crate::{
        frontmatter_file::{keeper::ArcMutex, FrontmatterFile, Keeper},
        frontmatter_query::{FrontmatterQuery, QueryValue, Scalar},
    };

    use super::{get_inner, post_inner};

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

    fn make_test_keeper() -> ArcMutex {
        let mut hm = HashMap::new();
        let mut fm = Mapping::new();
        fm.insert(
            serde_yaml::Value::String(s!("tag")),
            serde_yaml::Value::String(s!("blue")),
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
            path!("/blah.md"),
            FrontmatterFile {
                name: s!("blah.md"),
                frontmatter: Some(fm),
                body: s!(""),
                modified: dt!(2024, 1, 1, 16),
                created: dt!(2024, 1, 1, 15),
            },
        );
        ArcMutex::new(Keeper { inner: hm })
    }

    #[test]
    fn get() {
        let keeper = make_test_keeper();
        let mut params = HashMap::new();
        params.insert(s!("sort"), s!("created"));

        let (headers, _) = get_inner(&keeper, &params, "something.md").unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(Some("about.md"), prev_file_name);
        assert_eq!(None, next_file_name);

        let (headers, _) = get_inner(&keeper, &params, "about.md").unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(Some("blah.md"), prev_file_name);
        assert_eq!(Some("something.md"), next_file_name);

        let (headers, _) = get_inner(&keeper, &params, "blah.md").unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(None, prev_file_name);
        assert_eq!(Some("about.md"), next_file_name);
    }

    #[test]
    fn post() {
        let keeper = make_test_keeper();

        let mut params = HashMap::new();
        params.insert(s!("sort"), s!("created"));

        let mut query_inner = HashMap::new();
        query_inner.insert(s!("tag"), QueryValue::Scalar(Scalar::String(s!("blue"))));
        let query = FrontmatterQuery(query_inner);

        let (headers, _) = post_inner(&keeper, &params, "about.md", &query).unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(Some("blah.md"), prev_file_name);
        assert_eq!(None, next_file_name);

        let (headers, _) = post_inner(&keeper, &params, "blah.md", &query).unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(None, prev_file_name);
        assert_eq!(Some("about.md"), next_file_name);

        let (headers, _) = post_inner(&keeper, &params, "something.md", &query).unwrap();
        let next_file_name = headers
            .get("x-next-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        let prev_file_name = headers
            .get("x-prev-file")
            .map(|h| h.to_str())
            .transpose()
            .unwrap();
        assert_eq!(Some("about.md"), prev_file_name);
        assert_eq!(None, next_file_name);
    }
}
