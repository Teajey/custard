use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Number;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    String(String),
    Number(Number),
    Bool(bool),
    Null,
}

impl Scalar {
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

#[derive(Deserialize)]
#[serde(untagged)]
pub enum QueryValue {
    Vec(Vec<Scalar>),
    Scalar(Scalar),
}

impl QueryValue {
    pub fn is_subset(&self, fm_value: &serde_json::Value) -> bool {
        match (self, fm_value) {
            (QueryValue::Vec(vec), serde_json::Value::Array(fm_vec)) => {
                vec.iter().zip(fm_vec.iter()).all(|(s, fm)| s.matches(fm))
            }
            (QueryValue::Scalar(scalar), fm_scalar) => scalar.matches(fm_scalar),
            _ => false,
        }
    }
}

#[derive(Deserialize)]
pub struct FrontmatterQuery(HashMap<String, QueryValue>);

impl FrontmatterQuery {
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

    #[test]
    fn is_subset() {
        let frontmatter_query: FrontmatterQuery = deserial!({
            "salty": "pork"
        });

        assert!(frontmatter_query
            .is_subset(&deserial!({ "1": "this went in my mouth", "salty": "pork" })));
    }
}
