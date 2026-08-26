// hydragrow-backend/src/services/firebase_auth.rs
//! Xác thực Firebase ID token gửi từ Tauri desktop app.
//!
//! Luồng: client đăng nhập bằng Firebase Auth (email/password) ở frontend,
//! lấy ID token (JWT ký RS256), gửi lên backend qua header
//! `Authorization: Bearer <token>`. Middleware gọi `FirebaseAuthVerifier::verify`
//! để xác minh chữ ký + `iss`/`aud`/`exp` trước khi tin tưởng `sub` (Firebase UID).

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const JWKS_URL: &str =
    "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";
const JWKS_CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirebaseClaims {
    pub sub: String,
    pub email: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub aud: String,
    pub iss: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FirebaseAuthError {
    #[error("không lấy được public key của Firebase: {0}")]
    JwksFetch(String),
    #[error("token không hợp lệ: {0}")]
    InvalidToken(String),
    #[error("không tìm thấy public key phù hợp (kid={0})")]
    KeyNotFound(String),
}

#[derive(Default)]
struct JwksCache {
    jwks: Option<JwkSet>,
    fetched_at: Option<Instant>,
}

impl JwksCache {
    fn is_fresh(&self) -> bool {
        matches!(self.fetched_at, Some(at) if at.elapsed() < JWKS_CACHE_TTL)
    }
}

pub struct FirebaseAuthVerifier {
    project_id: String,
    cache: RwLock<JwksCache>,
    http: reqwest::Client,
}

impl FirebaseAuthVerifier {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            cache: RwLock::new(JwksCache::default()),
            http: reqwest::Client::new(),
        }
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, FirebaseAuthError> {
        let resp = self
            .http
            .get(JWKS_URL)
            .send()
            .await
            .map_err(|e| FirebaseAuthError::JwksFetch(e.to_string()))?;
        resp.json::<JwkSet>()
            .await
            .map_err(|e| FirebaseAuthError::JwksFetch(e.to_string()))
    }

    async fn get_jwks(&self, force_refresh: bool) -> Result<JwkSet, FirebaseAuthError> {
        if !force_refresh {
            let cache = self.cache.read().await;
            if cache.is_fresh()
                && let Some(jwks) = &cache.jwks
            {
                return Ok(jwks.clone());
            }
        }

        let jwks = self.fetch_jwks().await?;
        let mut cache = self.cache.write().await;
        cache.jwks = Some(jwks.clone());
        cache.fetched_at = Some(Instant::now());
        Ok(jwks)
    }

    /// Xác minh chữ ký RS256 + `iss`/`aud`/`exp` của Firebase ID token.
    /// Tự refetch JWKS một lần nếu `kid` không khớp cache (đề phòng Google vừa xoay khóa).
    pub async fn verify(&self, token: &str) -> Result<FirebaseClaims, FirebaseAuthError> {
        let header =
            decode_header(token).map_err(|e| FirebaseAuthError::InvalidToken(e.to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| FirebaseAuthError::InvalidToken("thiếu kid trong header".into()))?;

        let mut jwks = self.get_jwks(false).await?;
        let mut jwk = jwks.find(&kid).cloned();
        if jwk.is_none() {
            jwks = self.get_jwks(true).await?;
            jwk = jwks.find(&kid).cloned();
        }
        let jwk = jwk.ok_or_else(|| FirebaseAuthError::KeyNotFound(kid.clone()))?;

        let decoding_key = DecodingKey::from_jwk(&jwk).map_err(|e| {
            FirebaseAuthError::InvalidToken(format!("public key không hợp lệ: {e}"))
        })?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(std::slice::from_ref(&self.project_id));
        validation.set_issuer(&[format!(
            "https://securetoken.google.com/{}",
            self.project_id
        )]);

        let data = decode::<FirebaseClaims>(token, &decoding_key, &validation)
            .map_err(|e| FirebaseAuthError::InvalidToken(e.to_string()))?;

        if data.claims.sub.trim().is_empty() {
            return Err(FirebaseAuthError::InvalidToken("thiếu sub (uid)".into()));
        }

        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::jwk::{
        AlgorithmParameters, CommonParameters, Jwk, KeyAlgorithm, PublicKeyUse, RSAKeyParameters,
        RSAKeyType,
    };
    use jsonwebtoken::{EncodingKey, Header, encode};

    const TEST_PROJECT_ID: &str = "hydragrow-iot";
    const TEST_KID: &str = "test-key-1";

    // Cặp khóa RSA-2048 CHỈ DÙNG CHO TEST (tự sinh, không phải khóa thật của Firebase).
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCIPntYjm16yhGq
LxtJr+c7W3VjU21ME7EE16+fQE06nBVox/WTgUy1b917+edwJ2oGQ3EPSb1UdkT9
CnpEfjveXaiI0BuCqDTJfLUrIyY0FwOaz+7NPLAnEEHOTFo4SR/6QjrwnQE9LzbV
DfKYaRtVYswtkhpINPu797wqG/yqI58vReBs+WzQkMZ9OppnecKgIlmT/1lmJaDh
O/iyOdGxwZlNj9Lrc3oe0/8Ey+jo4T8vBsE7SLFJfgOiiiGo//Zov2kGtYi3MOjX
fmuEIAobdbYGTbuw9DfWLoNAy1JAs+PbdZWNV1xg16yMX/872zjFB6QMNgm7Xz1Q
0vozWY17AgMBAAECggEAB6RZjtIOWgDTlNQnl66CLdYnc3bOfqHsH/VpKGmW616t
5L6yi5+JCfRIXBfcX3IWhFtsEAt2zzIDFJ5t1UGvYf2m4mWp5V5B97tC/jRuhCV2
UkyvfFuXFdnXlxa5SMbxQDxOyghEdYYccT7jCKF8owFzqmqzhrFSHWz75PDE71YT
NvYfJDQpbRaNeUYkDBpoQC38KkV/73mRscgDFadN79qswQE/ghebZLKU/Qfz36PP
hwRcMhWWh4UAjgwAhuD6JuWCCTWc0bI+W6taJLG5SkcahXG1mqQgquWVMVvSZJpt
llsAowvKe1V3vfKmPRQR26rDZtCI/x+nKWoVNmveoQKBgQC+7EHEk+KcHlHuKauy
WXNYwFElBm3aQRlGoaEYxrlyCKO0A8U1l3a7FNREZPoQIQKf0LByEe8oF1M6CfES
P4wxJ7O2nRWMjaD6vnVIWnsVXU2qgvmgiMOuh4AXj4M4LyLEROpSi5uc0F3pbNNi
BoLrwK7AFf8lobA1HZQUfupojwKBgQC2rwF0jEELG4tFiRG9Rzz8xpc9cv0lqGzu
xKO6efWtcD9H7A39wc2Qxro/zf7pPo9KNay4AoFT6cXxN+XDuHWmEFIbzIodo1wd
CrW4eDVkclwUqt5XBhWQVuGrjqtJrGQfQUMvFgnrFMwAufxOHYGaMounZ1DPgqgv
9eAAsLzKVQKBgEHBBomYUSRpgNgge+Sp0AMSASBaTX0sjHL5+YyZ7IZsmUzHO6VQ
a/DUpKKFkGX7qHa4Hfy7Vn2dQumrQN6DClpnjQpooWJN6NJSw/vORbO+9Z+zChwS
3in+usviflPcUAH+piEVudtRG/bnpwmMqoxdSRIYwU4JmLFCZZyFdV/bAoGAQiQ6
sqfVJOBkHFj0Q0N3oU2FlIn9fZTtW8V2Qh3GBXWOc8vThPyWIMTSyicbE/fCiWvF
jRnbGTaapCtI1QQEFIv0LnxvxStQPnOSN6fOLP/6tsDmnztks03BhwuwmIwB9A78
9B9Wl/Z/pgOwhdfJBLsoNQQDDh6QJk0vPRDAScECgYEAu+kAJixrAbd2VCHus+sL
H47GbVf3oILflNW8DgSwqwqAXx2JqK4VjvuKryW+VDJC0IMJUsaGihLddttlbUAF
SlsvfTJpHsjN3glG9WeqIB1dOHFR0eXXP/NXbNO1ybthFYtwS3DSGG/dZ/R9lqOm
74jq7eaMyl2v44PLB0GAQjk=
-----END PRIVATE KEY-----";

    const TEST_MODULUS_B64: &str = "iD57WI5tesoRqi8bSa_nO1t1Y1NtTBOxBNevn0BNOpwVaMf1k4FMtW_de_nncCdqBkNxD0m9VHZE_Qp6RH473l2oiNAbgqg0yXy1KyMmNBcDms_uzTywJxBBzkxaOEkf-kI68J0BPS821Q3ymGkbVWLMLZIaSDT7u_e8Khv8qiOfL0XgbPls0JDGfTqaZ3nCoCJZk_9ZZiWg4Tv4sjnRscGZTY_S63N6HtP_BMvo6OE_LwbBO0ixSX4DooohqP_2aL9pBrWItzDo135rhCAKG3W2Bk27sPQ31i6DQMtSQLPj23WVjVdcYNesjF__O9s4xQekDDYJu189UNL6M1mNew";
    const TEST_EXPONENT_B64: &str = "AQAB";

    fn test_jwk_set() -> JwkSet {
        JwkSet {
            keys: vec![Jwk {
                common: CommonParameters {
                    public_key_use: Some(PublicKeyUse::Signature),
                    key_algorithm: Some(KeyAlgorithm::RS256),
                    key_id: Some(TEST_KID.to_string()),
                    ..Default::default()
                },
                algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                    key_type: RSAKeyType::RSA,
                    n: TEST_MODULUS_B64.to_string(),
                    e: TEST_EXPONENT_B64.to_string(),
                }),
            }],
        }
    }

    fn make_token(claims: &FirebaseClaims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(TEST_KID.to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY_PEM.as_bytes())
            .expect("test private key phải parse được");
        encode(&header, claims, &key).expect("ký test token phải thành công")
    }

    fn base_claims() -> FirebaseClaims {
        let now = jsonwebtoken::get_current_timestamp() as usize;
        FirebaseClaims {
            sub: "test-uid-123".to_string(),
            email: Some("someone@example.com".to_string()),
            iat: now,
            exp: now + 3600,
            aud: TEST_PROJECT_ID.to_string(),
            iss: format!("https://securetoken.google.com/{TEST_PROJECT_ID}"),
        }
    }

    async fn verifier_with_seeded_cache() -> FirebaseAuthVerifier {
        let verifier = FirebaseAuthVerifier::new(TEST_PROJECT_ID.to_string());
        let mut cache = verifier.cache.write().await;
        cache.jwks = Some(test_jwk_set());
        cache.fetched_at = Some(Instant::now());
        drop(cache);
        verifier
    }

    #[tokio::test]
    async fn accepts_valid_token_signed_by_known_key() {
        let verifier = verifier_with_seeded_cache().await;
        let token = make_token(&base_claims());

        let claims = verifier
            .verify(&token)
            .await
            .expect("token hợp lệ phải verify được");
        assert_eq!(claims.sub, "test-uid-123");
        assert_eq!(claims.email.as_deref(), Some("someone@example.com"));
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let verifier = verifier_with_seeded_cache().await;
        let mut claims = base_claims();
        let now = jsonwebtoken::get_current_timestamp() as usize;
        claims.iat = now - 7200;
        claims.exp = now - 3600;
        let token = make_token(&claims);

        let result = verifier.verify(&token).await;
        assert!(matches!(result, Err(FirebaseAuthError::InvalidToken(_))));
    }

    #[tokio::test]
    async fn rejects_wrong_audience() {
        let verifier = verifier_with_seeded_cache().await;
        let mut claims = base_claims();
        claims.aud = "some-other-project".to_string();
        let token = make_token(&claims);

        let result = verifier.verify(&token).await;
        assert!(matches!(result, Err(FirebaseAuthError::InvalidToken(_))));
    }

    #[tokio::test]
    async fn rejects_wrong_issuer() {
        let verifier = verifier_with_seeded_cache().await;
        let mut claims = base_claims();
        claims.iss = "https://securetoken.google.com/some-other-project".to_string();
        let token = make_token(&claims);

        let result = verifier.verify(&token).await;
        assert!(matches!(result, Err(FirebaseAuthError::InvalidToken(_))));
    }
}
