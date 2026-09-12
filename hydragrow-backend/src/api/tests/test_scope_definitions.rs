#[cfg(test)]
mod tests {
    use crate::api::scope_definitions::{KNOWN_SCOPES, is_valid_scope, scope_description};

    #[test]
    fn all_standard_scopes_are_valid() {
        for scope in KNOWN_SCOPES {
            assert!(is_valid_scope(scope), "Scope '{}' should be valid", scope);
        }
    }

    #[test]
    fn unknown_scope_is_invalid() {
        assert!(!is_valid_scope("device:nonexistent"));
        assert!(!is_valid_scope(""));
        assert!(!is_valid_scope("DEVICE:CONTROL")); // case-sensitive
    }

    #[test]
    fn wildcard_scope_is_valid() {
        assert!(is_valid_scope("*"));
    }

    #[test]
    fn events_write_scope_is_declared_and_described() {
        assert!(
            KNOWN_SCOPES.contains(&"events:write"),
            "events:write must be declared in KNOWN_SCOPES"
        );
        assert!(is_valid_scope("events:write"));
        assert_ne!(
            scope_description("events:write"),
            "Scope không xác định",
            "events:write must have a dedicated description"
        );
    }

    #[test]
    fn recipe_write_scope_is_declared_and_described() {
        assert!(
            KNOWN_SCOPES.contains(&"recipe:write"),
            "recipe:write must be declared in KNOWN_SCOPES"
        );
        assert!(is_valid_scope("recipe:write"));
        assert_ne!(
            scope_description("recipe:write"),
            "Scope không xác định",
            "recipe:write must have a dedicated description"
        );
    }
}
