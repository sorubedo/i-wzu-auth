use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct StoredConfig {
    username: String,
    password: String,
}

/// 获取配置文件路径：`$XDG_CONFIG_HOME/i-wzu-auth/config.json`
/// fallback: `~/.config/i-wzu-auth/config.json`
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("i-wzu-auth")
        .join("config.json")
}

/// 检查配置文件是否存在
pub fn config_exists() -> bool {
    config_path().exists()
}

/// 从配置文件读取用户名和密码
pub fn load_config() -> Result<(String, String), String> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).map_err(|e| {
        format!("无法读取配置文件 {}: {}", path.display(), e)
    })?;

    let config: StoredConfig =
        serde_json::from_str(&content).map_err(|e| format!("配置文件格式错误: {}", e))?;

    if config.username.is_empty() || config.password.is_empty() {
        return Err("配置文件中用户名或密码为空".to_string());
    }

    Ok((config.username, config.password))
}

/// 保存用户名和密码到配置文件
///
/// 负责创建目录、写入 JSON、设置文件权限为 0600
pub fn save_config(username: &str, password: &str) -> Result<(), String> {
    let path = config_path();

    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建配置目录 {}: {}", parent.display(), e))?;
    }

    let config = StoredConfig {
        username: username.to_string(),
        password: password.to_string(),
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("JSON 序列化失败: {}", e))?;

    std::fs::write(&path, &json)
        .map_err(|e| format!("无法写入配置文件 {}: {}", path.display(), e))?;

    // 设置文件权限为 0600 (仅 owner 可读写)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("无法设置配置文件权限: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let dir = std::env::temp_dir().join("i-wzu-auth-test");
        let _ = std::fs::create_dir_all(&dir);

        let path = dir.join("config.json");

        // 直接用内部函数测试（通过直接读写文件）
        let config = StoredConfig {
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        std::fs::write(&path, &json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: StoredConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.username, "test_user");
        assert_eq!(loaded.password, "test_password");

        // 清理
        let _ = std::fs::remove_dir_all(&dir);
    }
}
