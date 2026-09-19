use crate::spec::Endpoints;

#[derive(Debug, Clone, PartialEq)]
pub enum BreakingChange {
    RemovedEndpoint {
        path: String,
        method: String,
    },
    NewRequiredParameter {
        path: String,
        method: String,
        param: String,
    },
    ParameterBecameRequired {
        path: String,
        method: String,
        param: String,
    },
}

impl BreakingChange {
    pub fn describe(&self) -> String {
        match self {
            BreakingChange::RemovedEndpoint { path, method } => {
                format!("{} {path} was removed", method.to_uppercase())
            }
            BreakingChange::NewRequiredParameter {
                path,
                method,
                param,
            } => {
                format!(
                    "{} {path}: new required parameter '{param}' — existing clients don't send it",
                    method.to_uppercase()
                )
            }
            BreakingChange::ParameterBecameRequired {
                path,
                method,
                param,
            } => {
                format!(
                    "{} {path}: parameter '{param}' was optional, is now required",
                    method.to_uppercase()
                )
            }
        }
    }
}

/// Compares `old` against `new`, reporting only changes that can actually
/// break an existing client — not every difference (a new optional
/// parameter, a new endpoint, a parameter becoming *optional* are all
/// safe, additive changes and deliberately not reported here).
pub fn find_breaking_changes(old: &Endpoints, new: &Endpoints) -> Vec<BreakingChange> {
    let mut changes = Vec::new();

    for (path, method) in old.keys() {
        if !new.contains_key(&(path.clone(), method.clone())) {
            changes.push(BreakingChange::RemovedEndpoint {
                path: path.clone(),
                method: method.clone(),
            });
        }
    }

    for ((path, method), new_params) in new {
        let Some(old_params) = old.get(&(path.clone(), method.clone())) else {
            continue; // a brand-new endpoint — not breaking, nothing to compare against
        };
        for new_param in new_params {
            match old_params
                .iter()
                .find(|p| p.name == new_param.name && p.location == new_param.location)
            {
                None if new_param.required => {
                    changes.push(BreakingChange::NewRequiredParameter {
                        path: path.clone(),
                        method: method.clone(),
                        param: new_param.name.clone(),
                    });
                }
                Some(old_param) if !old_param.required && new_param.required => {
                    changes.push(BreakingChange::ParameterBecameRequired {
                        path: path.clone(),
                        method: method.clone(),
                        param: new_param.name.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::ParamInfo;

    fn endpoints(entries: Vec<((&str, &str), Vec<ParamInfo>)>) -> Endpoints {
        entries
            .into_iter()
            .map(|((p, m), params)| ((p.to_string(), m.to_string()), params))
            .collect()
    }

    fn param(name: &str, location: &str, required: bool) -> ParamInfo {
        ParamInfo {
            name: name.to_string(),
            location: location.to_string(),
            required,
        }
    }

    #[test]
    fn removed_endpoint_is_breaking() {
        let old = endpoints(vec![(("/users", "get"), vec![])]);
        let new = endpoints(vec![]);
        let changes = find_breaking_changes(&old, &new);
        assert_eq!(
            changes,
            vec![BreakingChange::RemovedEndpoint {
                path: "/users".to_string(),
                method: "get".to_string()
            }]
        );
    }

    #[test]
    fn new_endpoint_is_not_breaking() {
        let old = endpoints(vec![]);
        let new = endpoints(vec![(("/users", "get"), vec![])]);
        assert!(find_breaking_changes(&old, &new).is_empty());
    }

    #[test]
    fn new_required_parameter_is_breaking() {
        let old = endpoints(vec![(("/users", "post"), vec![])]);
        let new = endpoints(vec![(
            ("/users", "post"),
            vec![param("api_key", "header", true)],
        )]);
        let changes = find_breaking_changes(&old, &new);
        assert_eq!(
            changes,
            vec![BreakingChange::NewRequiredParameter {
                path: "/users".to_string(),
                method: "post".to_string(),
                param: "api_key".to_string()
            }]
        );
    }

    #[test]
    fn new_optional_parameter_is_not_breaking() {
        let old = endpoints(vec![(("/users", "get"), vec![])]);
        let new = endpoints(vec![(
            ("/users", "get"),
            vec![param("filter", "query", false)],
        )]);
        assert!(find_breaking_changes(&old, &new).is_empty());
    }

    #[test]
    fn parameter_becoming_required_is_breaking() {
        let old = endpoints(vec![(
            ("/users", "get"),
            vec![param("limit", "query", false)],
        )]);
        let new = endpoints(vec![(
            ("/users", "get"),
            vec![param("limit", "query", true)],
        )]);
        let changes = find_breaking_changes(&old, &new);
        assert_eq!(
            changes,
            vec![BreakingChange::ParameterBecameRequired {
                path: "/users".to_string(),
                method: "get".to_string(),
                param: "limit".to_string()
            }]
        );
    }

    #[test]
    fn parameter_becoming_optional_is_not_breaking() {
        let old = endpoints(vec![(
            ("/users", "get"),
            vec![param("limit", "query", true)],
        )]);
        let new = endpoints(vec![(
            ("/users", "get"),
            vec![param("limit", "query", false)],
        )]);
        assert!(find_breaking_changes(&old, &new).is_empty());
    }

    #[test]
    fn removed_parameter_is_not_breaking() {
        // A client that stops sending a now-gone parameter isn't broken.
        let old = endpoints(vec![(
            ("/users", "get"),
            vec![param("legacy_flag", "query", false)],
        )]);
        let new = endpoints(vec![(("/users", "get"), vec![])]);
        assert!(find_breaking_changes(&old, &new).is_empty());
    }

    #[test]
    fn identical_specs_have_no_breaking_changes() {
        let spec = endpoints(vec![(("/users", "get"), vec![param("id", "path", true)])]);
        assert!(find_breaking_changes(&spec, &spec).is_empty());
    }
}
