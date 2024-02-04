use serde::{de::DeserializeOwned, Serialize};

pub fn yaml_to_json<T: Serialize, U: DeserializeOwned>(yaml: T) -> U {
    serde_json::from_value(serde_json::to_value(yaml).expect("valid yaml must map to valid json"))
        .expect("Map<String, Value> is valid json")
}
