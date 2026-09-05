use std::collections::HashMap;

/// Thay thế mọi token `{{name}}` trong `template` bằng giá trị tương ứng
/// trong `vars`. Token không có trong `vars` được giữ nguyên `{{name}}` —
/// khớp hành vi preview phía frontend
/// (`renderTemplatePreview` trong hydragrow-frontend/src/lib/automation/templateVars.ts),
/// để không bao giờ âm thầm xoá 1 token chưa xác định khỏi cảnh báo thật.
pub fn render_alert_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("}}") else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let name = after_open[..end].trim();
        match vars.get(name) {
            Some(value) => out.push_str(value),
            None => {
                out.push_str("{{");
                out.push_str(&after_open[..end]);
                out.push_str("}}");
            }
        }
        rest = &after_open[end + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn substitutes_known_tokens() {
        let vars: HashMap<String, String> = [
            ("ec".to_string(), "1.8".to_string()),
            ("time".to_string(), "22:05:00 UTC".to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            render_alert_template("EC: {{ec}} lúc {{time}}", &vars),
            "EC: 1.8 lúc 22:05:00 UTC"
        );
    }

    #[test]
    fn leaves_unknown_tokens_untouched() {
        let vars = HashMap::new();
        assert_eq!(
            render_alert_template("Giá trị: {{unknown_var}}", &vars),
            "Giá trị: {{unknown_var}}"
        );
    }

    #[test]
    fn tolerates_whitespace_inside_braces() {
        let vars: HashMap<String, String> =
            [("ph_target_now".to_string(), "6.4".to_string())].into_iter().collect();
        assert_eq!(
            render_alert_template("giá trị {{ ph_target_now }}", &vars),
            "giá trị 6.4"
        );
    }

    #[test]
    fn returns_the_template_unchanged_when_it_has_no_tokens() {
        let vars = HashMap::new();
        assert_eq!(
            render_alert_template("không có biến nào", &vars),
            "không có biến nào"
        );
    }
}
