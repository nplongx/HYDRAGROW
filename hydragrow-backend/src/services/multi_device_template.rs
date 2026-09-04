use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Merge nông (shallow theo top-level key của ir_json: conditions/actions/trigger) —
/// key nào có trong `device_override` thì override thắng, key nào không có thì lấy từ template.
pub fn merge_template_with_override(template: &Value, device_override: &Value) -> Value {
    let mut result = template.clone();
    if let (Some(result_map), Some(override_map)) = (result.as_object_mut(), device_override.as_object()) {
        for (k, v) in override_map {
            result_map.insert(k.clone(), v.clone());
        }
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateTarget {
    pub device_id: String,
    pub overrides: Value, // {} nếu "giống gốc"
}

pub async fn apply_template(
    pool: &sqlx::PgPool,
    source: &crate::models::script::UserScript,
    targets: Vec<TemplateTarget>,
) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
    let mut applied_ids = Vec::new();
    for target in targets {
        let existing: Option<(uuid::Uuid, sqlx::types::Json<Value>)> = sqlx::query_as(
            "SELECT id, template_overrides FROM user_scripts \
             WHERE device_id = $1 AND template_source_id = $2",
        )
        .bind(&target.device_id)
        .bind(source.id)
        .fetch_optional(pool)
        .await?;

        let template_ir = source.ir_json.clone().unwrap_or(serde_json::json!({}));

        if let Some((existing_id, existing_overrides)) = existing {
            let merged = merge_template_with_override(&template_ir, &existing_overrides.0);
            sqlx::query("UPDATE user_scripts SET ir_json = $1 WHERE id = $2")
                .bind(&merged).bind(existing_id)
                .execute(pool).await?;
            applied_ids.push(existing_id);
        } else {
            let merged = merge_template_with_override(&template_ir, &target.overrides);
            let new_id = uuid::Uuid::new_v4();
            sqlx::query(
                "INSERT INTO user_scripts (id, device_id, kind, name, source, enabled, ir_json, next_flow_ids, template_source_id, template_overrides) \
                 VALUES ($1, $2, $3, $4, $5, TRUE, $6, '[]'::jsonb, $7, $8)",
            )
            .bind(new_id).bind(&target.device_id).bind(&source.kind).bind(&source.name)
            .bind(&source.source).bind(&merged).bind(source.id).bind(&target.overrides)
            .execute(pool).await?;
            applied_ids.push(new_id);
        }
    }
    Ok(applied_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_keeps_device_override_and_syncs_rest() {
        let template = json!({
            "conditions": [{"sensor":"ph","operator":">","value":7.5}],
            "actions": [{"type":"alert","level":"warning","message":"cao"}]
        });
        let device_override = json!({
            "conditions": [{"sensor":"ph","operator":">","value":8.0}]
        });
        let merged = merge_template_with_override(&template, &device_override);
        // override thắng ở field "conditions", nhưng "actions" đồng bộ từ template gốc
        assert_eq!(merged["conditions"][0]["value"], 8.0);
        assert_eq!(merged["actions"][0]["message"], "cao");
    }

    #[test]
    fn empty_override_fully_syncs_from_template() {
        let template = json!({"conditions": [{"sensor":"ec","operator":">","value":3.0}]});
        let merged = merge_template_with_override(&template, &json!({}));
        assert_eq!(merged, template);
    }
}
