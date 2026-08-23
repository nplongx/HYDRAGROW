#[cfg(test)]
mod tests {
    use crate::api::scope_definitions::{KNOWN_SCOPES, is_valid_scope};

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
}
