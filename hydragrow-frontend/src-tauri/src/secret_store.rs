use std::process::Command;

const SERVICE: &str = "com.hydragrow.frontend";
const API_KEY_ACCOUNT: &str = "api_key";

#[cfg(target_os = "macos")]
pub fn load_api_key() -> Result<String, String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            SERVICE,
            "-a",
            API_KEY_ACCOUNT,
            "-w",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}

#[cfg(target_os = "macos")]
pub fn save_api_key(api_key: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        let _ = Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                SERVICE,
                "-a",
                API_KEY_ACCOUNT,
            ])
            .output();
        return Ok(());
    }

    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-U",
            "-s",
            SERVICE,
            "-a",
            API_KEY_ACCOUNT,
            "-w",
            api_key.trim(),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Không thể lưu API key vào macOS Keychain".to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn load_api_key() -> Result<String, String> {
    let output = Command::new("cmdkey")
        .args(["/list:HydraGrowApiKey"])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::new())
    } else {
        Ok(String::new())
    }
}

#[cfg(target_os = "windows")]
pub fn save_api_key(api_key: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        let _ = Command::new("cmdkey")
            .args(["/delete:HydraGrowApiKey"])
            .output();
        return Ok(());
    }
    let status = Command::new("cmdkey")
        .args([
            "/generic:HydraGrowApiKey",
            &format!("/user:{}", API_KEY_ACCOUNT),
            &format!("/pass:{}", api_key.trim()),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Không thể lưu API key vào Windows Credential Manager".to_string())
    }
}

#[cfg(target_os = "linux")]
pub fn load_api_key() -> Result<String, String> {
    let output = Command::new("secret-tool")
        .args(["lookup", "service", SERVICE, "account", API_KEY_ACCOUNT])
        .output()
        .map_err(|e| {
            format!("Không tìm thấy secret-tool/libsecret để đọc OS credential vault: {e}")
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}

#[cfg(target_os = "linux")]
pub fn save_api_key(api_key: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        let _ = Command::new("secret-tool")
            .args(["clear", "service", SERVICE, "account", API_KEY_ACCOUNT])
            .output();
        return Ok(());
    }

    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label",
            "HydraGrow API Key",
            "service",
            SERVICE,
            "account",
            API_KEY_ACCOUNT,
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!("Không tìm thấy secret-tool/libsecret để lưu OS credential vault: {e}")
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(api_key.trim().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Không thể lưu API key vào libsecret credential vault".to_string())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn load_api_key() -> Result<String, String> {
    Ok(String::new())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn save_api_key(_api_key: &str) -> Result<(), String> {
    Err("Nền tảng này chưa hỗ trợ OS credential vault cho API key".to_string())
}
