use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Number;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Scalar {
    String(String),
    Number(Number),
    Bool(bool),
    Null,
}

impl Scalar {
    #[must_use]
    pub fn matches(&self, fm_scalar: &serde_json::Value) -> bool {
        match (self, fm_scalar) {
            (Scalar::String(a), serde_json::Value::String(b)) => a == b,
            (Scalar::Number(a), serde_json::Value::Number(b)) => a == b,
            (Scalar::Bool(a), serde_json::Value::Bool(b)) => a == b,
            (Scalar::Null, serde_json::Value::Null) => true,
            _ => false,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum QueryValue {
    Vec(Vec<Scalar>),
    Scalar(Scalar),
}

impl QueryValue {
    #[must_use]
    pub fn is_subset(&self, fm_value: &serde_json::Value) -> bool {
        match (self, fm_value) {
            (QueryValue::Vec(vec), serde_json::Value::Array(fm_vec)) => {
                vec.iter().all(|s| fm_vec.iter().any(|fm| s.matches(fm)))
            }
            (QueryValue::Scalar(scalar), fm_scalar) => scalar.matches(fm_scalar),
            _ => false,
        }
    }

    #[must_use]
    pub fn is_intersect(&self, fm_value: &serde_json::Value) -> bool {
        match (self, fm_value) {
            (QueryValue::Vec(vec), serde_json::Value::Array(fm_vec)) => {
                if vec.is_empty() {
                    return true;
                }
                vec.iter().any(|s| fm_vec.iter().any(|fm| s.matches(fm)))
            }
            (QueryValue::Scalar(scalar), fm_scalar) => scalar.matches(fm_scalar),
            _ => false,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct FrontmatterQuery(pub HashMap<String, QueryValue>);

impl FrontmatterQuery {
    #[must_use]
    pub fn is_subset(&self, json_frontmatter: &serde_json::Map<String, serde_json::Value>) -> bool {
        for (key, value) in &self.0 {
            let Some(fm_value) = json_frontmatter.get(key) else {
                return false;
            };

            if !value.is_subset(fm_value) {
                return false;
            }
        }

        true
    }

    #[must_use]
    pub fn is_intersect(
        &self,
        json_frontmatter: &serde_json::Map<String, serde_json::Value>,
    ) -> bool {
        for (key, value) in &self.0 {
            let Some(fm_value) = json_frontmatter.get(key) else {
                return false;
            };

            if !value.is_intersect(fm_value) {
                return false;
            }
        }

        true
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::FrontmatterQuery;

    macro_rules! deserial {
        ($tokens:tt) => {
            serde_json::from_value(json!($tokens)).unwrap()
        };
    }

    macro_rules! s {
        ($s:literal) => {
            Scalar::String($s.to_owned())
        };
    }

    #[test]
    fn is_subset() {
        let frontmatter_query: FrontmatterQuery = deserial!({
            "salty": "pork"
        });

        assert!(frontmatter_query
            .is_subset(&deserial!({ "1": "this went in my mouth", "salty": "pork" })));
    }

    #[test]
    fn is_intersect() {
        let frontmatter_query: FrontmatterQuery = deserial!({
            "listed": true,
            "tags": ["essay", "film"]
        });

        assert!(frontmatter_query.is_intersect(&deserial!({
            "listed": true,
            "tags": ["essay"]
        })));

        let frontmatter_query: FrontmatterQuery = deserial!({
            "listed": true,
            "tags": ["essay", "film"]
        });

        assert!(!frontmatter_query.is_intersect(&deserial!({
            "listed": true,
            "tags": ["story"]
        })));
    }

    mod query_value {
        use super::{
            super::{QueryValue, Scalar},
            json,
        };

        #[test]
        fn is_subset() {
            let a = QueryValue::Vec(vec![s!("dis")]);
            let b = json!(["dis", "a", "test", "gotta", "add", "more", "tags!"]);
            assert!(a.is_subset(&b), "first element match");

            let a = QueryValue::Vec(vec![s!("gotta")]);
            assert!(a.is_subset(&b), "later element match");

            let a = QueryValue::Vec(vec![s!("gotta"), s!("bucks")]);
            assert!(!a.is_subset(&b), "not all match elements match");

            let a = QueryValue::Vec(vec![]);
            assert!(a.is_subset(&b), "empty set is subset of all other sets");
        }

        #[test]
        fn is_intersect() {
            let a = QueryValue::Vec(vec![s!("dis")]);
            let b = json!(["dis", "a", "test", "gotta", "add", "more", "tags!"]);
            assert!(a.is_intersect(&b), "first element match");

            let a = QueryValue::Vec(vec![s!("gotta")]);
            assert!(a.is_intersect(&b), "later element match");

            let a = QueryValue::Vec(vec![s!("gotta"), s!("bucks")]);
            assert!(a.is_intersect(&b), "match despite strange element");

            let a = QueryValue::Vec(vec![]);
            assert!(
                a.is_intersect(&b),
                "empty set intersects with all other sets"
            );
        }
    }
}
