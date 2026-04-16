use crate::crypto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use ureq;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge: String,
    pub client_ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub res: String,
    pub error: Option<String>,
    pub error_msg: Option<String>,
}

pub struct SrunClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub ac_id: String,
}

impl SrunClient {
    pub fn new(base_url: &str, username: &str, password: &str, ac_id: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            ac_id: ac_id.to_string(),
        }
    }

    fn extract_jsonp(text: &str) -> Result<Value, String> {
        let re = Regex::new(r"^[^(]+\((.*)\)$").unwrap();
        if let Some(caps) = re.captures(text) {
            let json_str = caps.get(1).map_or("", |m| m.as_str());
            serde_json::from_str(json_str).map_err(|e| e.to_string())
        } else {
            let trimmed = text.trim();
            serde_json::from_str(trimmed).map_err(|e| e.to_string())
        }
    }

    pub fn check_online() -> bool {
        match ureq::get("http://www.google.cn/generate_204")
            .timeout(std::time::Duration::from_secs(3))
            .call() {
            Ok(resp) => resp.status() == 204,
            Err(_) => false,
        }
    }

    pub fn get_challenge(&self, ip: &str) -> Result<ChallengeResponse, String> {
        let url = format!("{}/cgi-bin/get_challenge", self.base_url);
        let resp = ureq::get(&url)
            .query("username", &self.username)
            .query("ip", ip)
            .query("callback", "jQuery123")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;

        let json = Self::extract_jsonp(&resp)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn login(&self) -> Result<AuthResponse, String> {
        let user_info = self.check_info("0.0.0.0")?;
        let ip = user_info["online_ip"].as_str()
            .or(user_info["client_ip"].as_str())
            .unwrap_or("0.0.0.0").to_string();
        let nas_ip = user_info["nas_ip"].as_str().unwrap_or("");

        let challenge = self.get_challenge(&ip)?;
        let token = challenge.challenge;
        let final_ip = if !challenge.client_ip.is_empty() { challenge.client_ip } else { ip };

        let hmd5 = crypto::hmac_md5(&token, &self.password);
        
        let info_str = format!(
            r#"{{"username":"{}","password":"{}","ip":"{}","acid":"{}","enc_ver":"srun_bx1"}}"#,
            self.username, self.password, final_ip, self.ac_id
        );
        let info_encrypted = crypto::xxtea_encode(&info_str, &token);
        let info_param = format!("{{SRBX1}}{}", crypto::custom_base64_encode(&info_encrypted));

        let n = "200";
        let auth_type = "1";
        
        let chksum_str = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            token, self.username,
            token, hmd5,
            token, self.ac_id,
            token, final_ip,
            token, n,
            token, auth_type,
            token, info_param
        );
        let chksum = crypto::sha1(&chksum_str);

        let url = format!("{}/cgi-bin/srun_portal", self.base_url);
        let resp = ureq::get(&url)
            .query("action", "login")
            .query("username", &self.username)
            .query("password", &format!("{{MD5}}{}", hmd5))
            .query("ac_id", &self.ac_id)
            .query("ip", &final_ip)
            .query("info", &info_param)
            .query("chksum", &chksum)
            .query("n", n)
            .query("type", auth_type)
            .query("os", "Linux")
            .query("name", "Linux")
            .query("double_stack", "0")
            .query("nas_ip", nas_ip)
            .query("callback", "jQuery123")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;

        let json = Self::extract_jsonp(&resp)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn logout(&self) -> Result<AuthResponse, String> {
        let user_info = self.check_info("0.0.0.0")?;
        
        let ip = user_info["online_ip"].as_str()
            .or(user_info["client_ip"].as_str())
            .unwrap_or("0.0.0.0");

        // 获取不带域名的用户名和单独的域名
        let user_name_only = user_info["user_name"].as_str().unwrap_or(&self.username);
        let domain = user_info["domain"].as_str().unwrap_or("");
        
        // 构造带域名的完整用户名 (用于 srun_portal)
        let username_with_domain = if !domain.is_empty() {
            format!("{}@{}", user_name_only, domain)
        } else {
            user_name_only.to_string()
        };

        // 步骤 1: 解除 MAC 无感绑定 (Logout DM / Unbind)
        // 参考 portal.js 的 _logoutDm 实现：使用不带域名的用户名，且在 MacAuth 模式下仅需此步
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        
        let unbind = "1";
        // 签名逻辑使用不带域名的用户名: sha1(time + user_name_only + ip + unbind + time)
        let sign_data = format!("{}{}{}{}{}", time, user_name_only, ip, unbind, time);
        let sign = crypto::sha1(&sign_data);

        let url_dm = format!("{}/cgi-bin/rad_user_dm", self.base_url);
        let resp_dm_str = ureq::get(&url_dm)
            .query("username", user_name_only)
            .query("ip", ip)
            .query("unbind", unbind)
            .query("time", &time)
            .query("sign", &sign)
            .query("callback", "jQuery123")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;

        // 步骤 2: 标准 Session 注销 (Logout Normal) - 作为补充调用
        let url_logout = format!("{}/cgi-bin/srun_portal", self.base_url);
        let _ = ureq::get(&url_logout)
            .query("action", "logout")
            .query("username", &username_with_domain)
            .query("ip", ip)
            .query("ac_id", &self.ac_id)
            .query("double_stack", "0")
            .query("callback", "jQuery123")
            .call();

        let json = Self::extract_jsonp(&resp_dm_str)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn check_info(&self, ip: &str) -> Result<Value, String> {
        let url = format!("{}/cgi-bin/rad_user_info", self.base_url);
        let resp = ureq::get(&url)
            .query("ip", ip)
            .query("callback", "jQuery123")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;

        Self::extract_jsonp(&resp)
    }
}
