use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::Value;

const HTTP_METHODS: &[&str] = &[
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParamInfo {
    pub name: String,
    pub location: String,
    pub required: bool,
}

/// `(path, method) -> parameters`. A pragmatic, partial OpenAPI model —
/// not a full spec-compliant deserializer (no `$ref` resolution, no
/// `components` handling). Reasonable for what this tool actually checks
/// (path/method/parameter-required existence), and an honest scope
/// smaller than a real OpenAPI library would cover — see README.
pub type Endpoints = BTreeMap<(String, String), Vec<ParamInfo>>;

pub fn parse_spec(json: &str) -> Result<Endpoints> {
    let root: Value = serde_json::from_str(json).context("not valid JSON")?;
    let paths = root
        .get("paths")
        .and_then(Value::as_object)
        .context("no top-level 'paths' object — not an OpenAPI spec?")?;

    let mut endpoints = Endpoints::new();
    for (path, methods) in paths {
        let Some(methods_obj) = methods.as_object() else {
            continue;
        };
        for (method, operation) in methods_obj {
            let method_lower = method.to_ascii_lowercase();
            if !HTTP_METHODS.contains(&method_lower.as_str()) {
                continue; // e.g. "parameters" or "$ref" siblings at the path-item level
            }
            let params = extract_parameters(operation);
            endpoints.insert((path.clone(), method_lower), params);
        }
    }
    Ok(endpoints)
}

fn extract_parameters(operation: &Value) -> Vec<ParamInfo> {
    let Some(params) = operation.get("parameters").and_then(Value::as_array) else {
        return Vec::new();
    };
    params
        .iter()
        .filter_map(|p| {
            // A bare `{"$ref": "..."}` parameter isn't resolved in this v1
            // (see module doc) — skipped rather than guessed at.
            let name = p.get("name")?.as_str()?.to_string();
            let location = p.get("in")?.as_str()?.to_string();
            let required = p.get("required").and_then(Value::as_bool).unwrap_or(false);
            Some(ParamInfo {
                name,
                location,
                required,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "openapi": "3.0.0",
        "paths": {
            "/users": {
                "get": {
                    "parameters": [
                        {"name": "limit", "in": "query", "required": false},
                        {"name": "X-Api-Key", "in": "header", "required": true}
                    ]
                },
                "post": {}
            },
            "/users/{id}": {
                "get": {
                    "parameters": [{"name": "id", "in": "path", "required": true}]
                }
            }
        }
    }"#;

    #[test]
    fn parses_paths_and_methods() {
        let endpoints = parse_spec(SAMPLE).unwrap();
        assert!(endpoints.contains_key(&("/users".to_string(), "get".to_string())));
        assert!(endpoints.contains_key(&("/users".to_string(), "post".to_string())));
        assert!(endpoints.contains_key(&("/users/{id}".to_string(), "get".to_string())));
    }

    #[test]
    fn parses_parameters_with_required_flag() {
        let endpoints = parse_spec(SAMPLE).unwrap();
        let params = &endpoints[&("/users".to_string(), "get".to_string())];
        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "limit" && !p.required));
        assert!(params.iter().any(|p| p.name == "X-Api-Key" && p.required));
    }

    #[test]
    fn operation_with_no_parameters_key_is_an_empty_list_not_an_error() {
        let endpoints = parse_spec(SAMPLE).unwrap();
        assert_eq!(
            endpoints[&("/users".to_string(), "post".to_string())],
            vec![]
        );
    }

    #[test]
    fn non_method_keys_at_the_path_item_level_are_ignored() {
        let spec = r#"{"paths": {"/x": {"parameters": [], "get": {}}}}"#;
        let endpoints = parse_spec(spec).unwrap();
        assert_eq!(endpoints.len(), 1);
        assert!(endpoints.contains_key(&("/x".to_string(), "get".to_string())));
    }

    #[test]
    fn missing_paths_object_is_a_clean_error() {
        assert!(parse_spec(r#"{"openapi": "3.0.0"}"#).is_err());
    }

    #[test]
    fn invalid_json_is_a_clean_error() {
        assert!(parse_spec("not json at all").is_err());
    }
}
