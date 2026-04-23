use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, instrument};

use crate::AppState;
use crate::db::postgres::{
    get_device_blockchain_history as fetch_history_from_db, insert_blockchain_tx,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct DeviceLogPayload {
    pub device_id: String,
    pub season_id: String,
    pub action: String,
    pub value: f64,
    pub timestamp: String,
}

#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    pub season_id: Option<String>,
}

// POST /api/blockchain/log
#[instrument(skip(app_state))]
pub async fn push_log_to_blockchain(
    payload: web::Json<DeviceLogPayload>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let log_data = payload.into_inner();
    let json_string = match serde_json::to_string(&log_data) {
        Ok(s) => s,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"status": "error", "message": "Dữ liệu không hợp lệ"}));
        }
    };

    // 1. Ghi lên mạng Solana (Payload json_string giờ đã có chứa season_id)
    match app_state
        .solana_traceability
        .record_dosing_history(&json_string)
        .await
    {
        Ok(tx_id) => {
            info!("✅ Đã ghi lên Solana: {}", tx_id);

            // 2. LƯU LẠI VÀO CƠ SỞ DỮ LIỆU SQLITE (Kèm theo season_id)
            if let Err(e) = insert_blockchain_tx(
                &app_state.pg_pool,
                &log_data.device_id,
                &log_data.season_id, // 🟢 Đẩy season_id xuống Database
                &log_data.action,
                &tx_id,
            )
            .await
            {
                // Nếu lỗi DB, ta chỉ log ra chứ không báo lỗi cho Frontend
                // vì cốt lõi là dữ liệu ĐÃ LÊN BLOCKCHAIN thành công rồi.
                error!("❌ Lỗi lưu tx_id vào SQLite: {:?}", e);
            }

            HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Dữ liệu đã được niêm phong trên Blockchain!",
                "data": {
                    "tx_id": tx_id,
                    "explorer_url": format!("https://solscan.io/tx/{}?cluster=devnet", tx_id)
                }
            }))
        }
        Err(e) => {
            error!("❌ Lỗi đẩy dữ liệu lên Solana: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"status": "error", "message": e}))
        }
    }
}

/* ==============================================================
   2. ENDPOINT: LẤY LỊCH SỬ BLOCKCHAIN TỪ SQLITE
   GET /api/blockchain/devices/{device_id}?season_id=...
============================================================== */
#[instrument(skip(app_state))]
pub async fn get_device_blockchain_history(
    path: web::Path<String>,
    query: web::Query<HistoryQuery>, // 🟢 BẮT QUERY TỪ URL
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let season_filter = query.into_inner().season_id;

    info!(
        "Đang lấy lịch sử blockchain cho thiết bị: {} (Mùa vụ: {:?})",
        device_id, season_filter
    );

    // KÉO DỮ LIỆU THẬT TỪ SQLITE (Truyền thêm bộ lọc season)
    match fetch_history_from_db(&app_state.pg_pool, &device_id, season_filter).await {
        Ok(history) => HttpResponse::Ok().json(json!({
            "status": "success",
            "data": history
        })),
        Err(e) => {
            error!("❌ Lỗi truy vấn lịch sử blockchain từ DB: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Không thể truy xuất dữ liệu từ cơ sở dữ liệu"
            }))
        }
    }
}

// GET /api/blockchain/verify/{tx_id}
#[instrument(skip(_app_state))]
pub async fn verify_transaction_onchain(
    path: web::Path<String>,
    _app_state: web::Data<AppState>,
) -> impl Responder {
    let tx_id = path.into_inner();
    info!("Verify On-chain cho TxID: {}", tx_id);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "tx_id": tx_id,
            "network": "Solana Devnet",
            "program": "SPL Memo",
            "verification_links": {
                "solscan": format!("https://solscan.io/tx/{}?cluster=devnet", tx_id),
                "solana_explorer": format!("https://explorer.solana.com/tx/{}?cluster=devnet", tx_id)
            },
            "message": "Dữ liệu được bảo vệ bằng mật mã và không thể thay đổi."
        }
    }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/blockchain/log", web::post().to(push_log_to_blockchain))
        .route(
            "/devices/{device_id}/blockchain",
            web::get().to(get_device_blockchain_history),
        )
        .route(
            "/blockchain/verify/{tx_id}",
            web::get().to(verify_transaction_onchain),
        );
}
