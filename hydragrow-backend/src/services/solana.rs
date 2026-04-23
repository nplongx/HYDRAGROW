use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction}, // Thêm AccountMeta
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};

pub struct SolanaTraceability {
    client: Arc<RpcClient>,
    keypair: Keypair,
}

impl SolanaTraceability {
    pub fn new(rpc_url: &str, private_key_bytes: &[u8]) -> Self {
        let client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

        let keypair = Keypair::try_from(private_key_bytes)
            .expect("Lỗi: Không thể khởi tạo Ví Solana từ Private Key cung cấp!");

        Self {
            client: Arc::new(client),
            keypair,
        }
    }

    pub async fn record_dosing_history(&self, json_payload: &str) -> Result<String, String> {
        let memo_program_id =
            Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap();

        let memo_ix = Instruction {
            program_id: memo_program_id,
            accounts: vec![AccountMeta::new_readonly(self.keypair.pubkey(), true)],
            data: json_payload.as_bytes().to_vec(),
        };

        let recent_blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(|e| format!("Lỗi lấy Blockhash: {}", e))?;

        let transaction = Transaction::new_signed_with_payer(
            &[memo_ix],                   // Danh sách các lệnh (ở đây chỉ có 1 lệnh ghi Memo)
            Some(&self.keypair.pubkey()), // Ai là người trả phí Gas? (Ví server)
            &[&self.keypair],             // Ai là người ký? (Ví server)
            recent_blockhash,
        );

        // 4. Gửi giao dịch
        info!("Đang gửi dữ liệu lên Solana Blockchain...");
        match self.client.send_and_confirm_transaction(&transaction).await {
            Ok(signature) => {
                let tx_id = signature.to_string();
                info!("✅ Thành công! Đã lưu log thiết bị lên Blockchain.");
                info!("🔍 Xem tại: https://solscan.io/tx/{}?cluster=devnet", tx_id);
                Ok(tx_id)
            }
            Err(e) => {
                error!("❌ Lỗi gửi giao dịch Solana: {}", e);
                Err(e.to_string())
            }
        }
    }
}
