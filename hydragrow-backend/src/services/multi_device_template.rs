use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Merge nông (shallow theo top-level key của ir_json: conditions/actions/trigger) —
/// key nào có trong `device_override` thì override thắng, key nào không có thì lấy từ template.
pub fn merge_template_with_override(template: &Value, device_override: &Value) -> Value {
    let mut result = template.clone();
    if let (Some(result_map), Some(override_map)) =
        (result.as_object_mut(), device_override.as_object())
    {
        for (k, v) in override_map {
            if v.is_null() {
                result_map.remove(k);
            } else {
                result_map.insert(k.clone(), v.clone());
            }
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

        if let Some((existing_id, _stale_overrides)) = existing {
            let merged = merge_template_with_override(&template_ir, &target.overrides);
            sqlx::query("UPDATE user_scripts SET ir_json = $1, template_overrides = $2 WHERE id = $3")
                .bind(&merged)
                .bind(&target.overrides)
                .bind(existing_id)
                .execute(pool)
                .await?;
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

    #[test]
    fn a_null_override_value_deletes_that_key_instead_of_setting_it_to_null() {
        let template = json!({"conditions": [1], "actions": [2]});
        let device_override = json!({"actions": null});
        let merged = merge_template_with_override(&template, &device_override);
        assert!(!merged.as_object().unwrap().contains_key("actions"));
        assert_eq!(merged["conditions"], json!([1]));
    }

    #[test]
    fn merge_prefers_current_request_overrides_over_stale_stored_ones() {
        // Pure merge-level: the already-linked branch must merge with the
        // CURRENT request's overrides, not the stale stored ones.
        let template_ir = json!({"conditions": [1], "actions": ["old"]});
        let stale = json!({"actions": ["stale"]});
        let current = json!({"actions": ["current"]});
        let with_stale = merge_template_with_override(&template_ir, &stale);
        let with_current = merge_template_with_override(&template_ir, &current);
        assert_eq!(with_stale["actions"], json!(["stale"]));
        assert_eq!(with_current["actions"], json!(["current"]));
        assert_ne!(with_stale, with_current);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn apply_template_reapply_respects_the_current_requests_preserve_choice(
        pool: sqlx::PgPool,
    ) {
        use crate::models::script::UserScript;
        use chrono::Utc;
        let source = UserScript {
            id: uuid::Uuid::new_v4(),
            device_id: "src-dev".to_string(),
            kind: "alert".to_string(),
            name: "tpl".to_string(),
            source: "fn main(input) {}".to_string(),
            enabled: true,
            ir_json: Some(json!({
                "conditions": [{"sensor":"ec","operator":">","value":3.0}],
                "configOverwrite": {"configKey":"ec_target","value":"2.4"}
            })),
            next_flow_ids: vec![],
            cron_next_run_at: None,
            template_source_id: None,
            template_overrides: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        sqlx::query(
            "INSERT INTO user_scripts (id, device_id, kind, name, source, enabled, ir_json, next_flow_ids, template_source_id, template_overrides) \
             VALUES ($1,$2,$3,$4,$5,TRUE,$6,'[]'::jsonb,$7,$8)",
        )
        .bind(source.id).bind(&source.device_id).bind(&source.kind).bind(&source.name)
        .bind(&source.source).bind(source.ir_json.clone().unwrap()).bind(source.template_source_id).bind(serde_json::json!({}))
        .execute(&pool).await.unwrap();
        // First apply with empty overrides → linked script keeps configOverwrite.
        let first_ids = apply_template(&pool, &source, vec![TemplateTarget {
            device_id: "dev-target".to_string(),
            overrides: json!({}),
        }]).await.unwrap();
        let row1: (serde_json::Value,) = sqlx::query_as(
            "SELECT ir_json FROM user_scripts WHERE device_id = 'dev-target' AND template_source_id = $1",
        )
        .bind(source.id)
        .fetch_one(&pool).await.unwrap();
        assert_eq!(row1.0["configOverwrite"], json!({"configKey":"ec_target","value":"2.4"}));
        // Second apply with the null tombstone → same script id, configOverwrite
        // REMOVED, other template keys (conditions) still synced.
        let second_ids = apply_template(&pool, &source, vec![TemplateTarget {
            device_id: "dev-target".to_string(),
            overrides: json!({"configOverwrite": null}),
        }]).await.unwrap();
        assert_eq!(first_ids, second_ids);
        let row2: (serde_json::Value, serde_json::Value) = sqlx::query_as(
            "SELECT ir_json, template_overrides FROM user_scripts WHERE device_id = 'dev-target' AND template_source_id = $1",
        )
        .bind(source.id)
        .fetch_one(&pool).await.unwrap();
        assert!(!row2.0.as_object().unwrap().contains_key("configOverwrite"));
        assert_eq!(row2.0["conditions"], json!([{"sensor":"ec","operator":">","value":3.0}]));
        assert_eq!(row2.1, json!({"configOverwrite": null}));
    }
}
