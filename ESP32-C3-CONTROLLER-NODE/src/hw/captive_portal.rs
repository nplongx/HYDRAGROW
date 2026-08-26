//! AP fallback + captive portal để nhập WiFi credentials lần đầu.
//! Mở AP "HydraGrow-Setup", phục vụ web form tại 192.168.4.1:80.

use anyhow::{anyhow, Result};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use hydragrow_shared::{WifiCandidate, WifiCredentialList};
use log::{info, warn};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const HTML: &str = r#"HTTP/1.1 200 OK
Content-Type: text/html

<!DOCTYPE html><html><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>HydraGrow WiFi Setup</title>
<style>body{font-family:sans-serif;max-width:400px;margin:40px auto;padding:16px;}
input,button{width:100%;padding:10px;margin:8px 0;box-sizing:border-box;font-size:16px;}
button{background:#2563eb;color:#fff;border:none;border-radius:6px;cursor:pointer;}
</style></head><body>
<h2>HydraGrow — Thiet Lap WiFi</h2>
<form method="POST" action="/save">
  <label>SSID</label>
  <input type="text" name="ssid" placeholder="Ten WiFi" required>
  <label>Mat khau</label>
  <input type="password" name="pass" placeholder="Mat khau WiFi">
  <label>Uu tien (0=cao nhat)</label>
  <input type="number" name="priority" value="0" min="0" max="255">
  <button type="submit">Luu &amp; Ket Noi</button>
</form>
</body></html>
"#;

const SUCCESS_HTML: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h2>Da luu! Thiet bi se khoi dong lai.</h2>";
const FAIL_HTML: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h2>Khong ket noi duoc. Thu lai.</h2><a href='/'>Quay lai</a>";

/// URL-decode a percent-encoded string (+ → space, %XX → char).
fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '+' => out.push(' '),
            '%' => {
                let h1 = chars.next().unwrap_or('0');
                let h2 = chars.next().unwrap_or('0');
                let hex = format!("{}{}", h1, h2);
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    out.push(byte as char);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Extract `key=value` from URL-encoded body.
fn extract_field<'a>(body: &'a str, key: &str) -> &'a str {
    let prefix = format!("{}=", key);
    body.split('&')
        .find(|part| part.starts_with(&prefix))
        .map(|part| &part[prefix.len()..])
        .unwrap_or("")
}

/// Parse POST body → (ssid, password, priority). Returns None if SSID empty.
pub fn parse_form_body(body: &str) -> Option<(String, String, u8)> {
    let ssid = url_decode(extract_field(body, "ssid"));
    if ssid.trim().is_empty() {
        return None;
    }
    let pass = url_decode(extract_field(body, "pass"));
    let priority = extract_field(body, "priority").parse::<u8>().unwrap_or(0);
    Some((ssid, pass, priority))
}

fn handle_client(mut stream: TcpStream, buf: &mut [u8]) -> Option<(String, String, u8)> {
    let n = stream.read(buf).unwrap_or(0);
    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let is_post = request.starts_with("POST");

    if is_post {
        // Body bắt đầu sau \r\n\r\n
        if let Some(body_start) = request.find("\r\n\r\n") {
            let body = &request[body_start + 4..];
            if let Some(fields) = parse_form_body(body) {
                let _ = stream.write_all(SUCCESS_HTML.as_bytes());
                return Some(fields);
            }
        }
        let _ = stream.write_all(FAIL_HTML.as_bytes());
    } else {
        let _ = stream.write_all(HTML.as_bytes());
    }
    None
}

/// Mở softAP và chạy captive portal.
/// Blocking cho đến khi credentials được lưu, hoặc `timeout` hết (None = vô hạn).
/// Trả về true nếu đã lưu credentials mới.
pub fn run_captive_portal(
    nvs_partition: EspDefaultNvsPartition,
    timeout: Option<Duration>,
) -> Result<bool> {
    info!("🌐 [PORTAL] Mở AP 'HydraGrow-Setup' tại 192.168.4.1");

    let listener =
        TcpListener::bind("0.0.0.0:80").map_err(|e| anyhow!("Cannot bind TCP 80: {e}"))?;
    listener.set_nonblocking(true)?;

    let deadline = timeout.map(|d| Instant::now() + d);
    let mut buf = vec![0u8; 4096];

    loop {
        if let Some(dl) = deadline {
            if Instant::now() > dl {
                warn!("[PORTAL] Timeout hết. Thoát captive portal.");
                return Ok(false);
            }
        }

        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                if let Some((ssid, pass, priority)) = handle_client(stream, &mut buf) {
                    info!(
                        "[PORTAL] Đã nhận credentials SSID='{}', priority={}",
                        ssid, priority
                    );
                    // Ghi vào NVS
                    let mut current = crate::hw::load_wifi_list(nvs_partition.clone());
                    let candidates = &mut current.candidates;
                    // Không duplicate SSID
                    candidates.retain(|c: &WifiCandidate| c.ssid != ssid);
                    candidates.push(WifiCandidate {
                        ssid,
                        password: pass,
                        priority,
                    });
                    let new_list = WifiCredentialList {
                        candidates: candidates.clone(),
                    };
                    if let Ok(mut nvs) =
                        esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), "agitech", true)
                    {
                        if let Err(e) = crate::hw::save_wifi_list(&mut nvs, &new_list) {
                            warn!("[PORTAL] Lỗi lưu NVS: {:?}", e);
                        }
                    }
                    return Ok(true);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                warn!("[PORTAL] TCP error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::parse_form_body;

    #[test]
    fn parse_form_body_extracts_ssid_and_password() {
        let body = "ssid=MyNetwork&pass=secret123&priority=1";
        let (ssid, pass, priority) = super::parse_form_body(body).unwrap();
        assert_eq!(ssid, "MyNetwork");
        assert_eq!(pass, "secret123");
        assert_eq!(priority, 1u8);
    }

    #[test]
    fn parse_form_body_url_decodes_spaces() {
        let body = "ssid=My+Network&pass=my%20pass&priority=0";
        let (ssid, pass, _) = super::parse_form_body(body).unwrap();
        assert_eq!(ssid, "My Network");
        assert_eq!(pass, "my pass");
    }

    #[test]
    fn parse_form_body_returns_none_on_empty_ssid() {
        let body = "ssid=&pass=secret&priority=0";
        assert!(super::parse_form_body(body).is_none());
    }
}
