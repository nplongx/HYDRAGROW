use crate::api::middleware::auth::AuthContext;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct TopicStatus {
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TopicRow {
    pub device_id: String,
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

pub fn group_by_device(rows: Vec<TopicRow>) -> HashMap<String, Vec<TopicStatus>> {
    let mut map: HashMap<String, Vec<TopicStatus>> = HashMap::new();
    for row in rows {
        map.entry(row.device_id).or_default().push(TopicStatus {
            topic_category: row.topic_category,
            last_seen_at: row.last_seen_at,
        });
    }
    map
}

pub fn has_health_read_scope(auth: &AuthContext) -> bool {
    auth.has_scope("health:read")
}

fn auth_or_forbidden(req: &HttpRequest) -> Result<AuthContext, HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !has_health_read_scope(&auth) {
        return Err(HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "health:read"
        })));
    }
    Ok(auth)
}

pub async fn get_all_health_topics(req: HttpRequest) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }
    let grouped: HashMap<String, Vec<TopicStatus>> = HashMap::new();
    HttpResponse::Ok().json(json!({ "status": "success", "data": grouped }))
}

pub async fn get_device_health_topics(
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }
    let _device_id = path.into_inner();
    let topics: Vec<TopicStatus> = Vec::new();
    HttpResponse::Ok().json(json!({ "status": "success", "data": topics }))
}

pub fn init_fleet_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/topics", web::get().to(get_all_health_topics));
}

pub fn init_device_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/topics", web::get().to(get_device_health_topics));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn topic_status_serializes_last_seen_as_rfc3339() {
        let ts = Utc.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap();
        let status = TopicStatus {
            topic_category: "sensors".to_string(),
            last_seen_at: ts,
        };
        let v = serde_json::to_value(&status).unwrap();
        let s = v.get("last_seen_at").unwrap().as_str().unwrap();
        assert!(s.starts_with("2024-05-01T12:00:00"));
        let parsed: DateTime<Utc> = s.parse().unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn group_by_device_groups_multiple_topics_under_one_device_id() {
        let ts = Utc.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap();
        let rows = vec![
            TopicRow {
                device_id: "dev-01".to_string(),
                topic_category: "sensors".to_string(),
                last_seen_at: ts,
            },
            TopicRow {
                device_id: "dev-01".to_string(),
                topic_category: "status".to_string(),
                last_seen_at: ts,
            },
            TopicRow {
                device_id: "dev-02".to_string(),
                topic_category: "sensors".to_string(),
                last_seen_at: ts,
            },
        ];
        let grouped = group_by_device(rows);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("dev-01").unwrap().len(), 2);
        assert_eq!(grouped.get("dev-02").unwrap().len(), 1);
    }
}
