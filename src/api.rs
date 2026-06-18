use crate::crypto;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub double_stack: bool,
    client: reqwest::blocking::Client,
}

impl SrunClient {
    pub fn new(
        base_url: &str,
        username: &str,
        password: &str,
        ac_id: &str,
        double_stack: bool,
        interface: Option<&str>,
    ) -> Self {
        let mut builder = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .tcp_nodelay(true);

        if let Some(iface) = interface {
            builder = builder.interface(iface);
        }

        let client = builder.build().expect("创建 HTTP 客户端失败");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            ac_id: ac_id.to_string(),
            double_stack,
            client,
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

    /// 联网检测。如果指定了 `interface`，则通过该网卡发送检测请求；
    /// 否则使用默认路由（可能被 TUN 代理劫持）。
    pub fn check_online(check_url: &str, interface: Option<&str>) -> bool {
        let result = match interface {
            Some(iface) => {
                let client = reqwest::blocking::Client::builder()
                    .interface(iface)
                    .timeout(std::time::Duration::from_secs(3))
                    .build();
                match client {
                    Ok(c) => c.get(check_url).send(),
                    Err(_) => return false,
                }
            }
            None => {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(3))
                    .build();
                match client {
                    Ok(c) => c.get(check_url).send(),
                    Err(_) => return false,
                }
            }
        };
        match result {
            Ok(resp) => resp.status() == 204,
            Err(_) => false,
        }
    }

    pub fn get_challenge(&self, ip: &str) -> Result<ChallengeResponse, String> {
        let url = format!("{}/cgi-bin/get_challenge", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("username", self.username.as_str()),
                ("ip", ip),
                ("callback", "jQuery123"),
            ])
            .send()
            .map_err(|e| e.to_string())?
            .text()
            .map_err(|e| e.to_string())?;

        let json = Self::extract_jsonp(&resp)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn login(&self) -> Result<AuthResponse, String> {
        let user_info = self.check_info("")?;
        let ip = user_info["online_ip"]
            .as_str()
            .or(user_info["client_ip"].as_str())
            .unwrap_or("0.0.0.0")
            .to_string();
        let nas_ip = user_info["nas_ip"].as_str().unwrap_or("");

        let challenge = self.get_challenge(&ip)?;
        let token = challenge.challenge;
        let final_ip = if !challenge.client_ip.is_empty() {
            challenge.client_ip
        } else {
            ip
        };

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
            token,
            self.username,
            token,
            hmd5,
            token,
            self.ac_id,
            token,
            final_ip,
            token,
            n,
            token,
            auth_type,
            token,
            info_param
        );
        let chksum = crypto::sha1(&chksum_str);

        let os_name = if cfg!(target_os = "windows") {
            "Windows 10"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else if cfg!(target_os = "android") {
            "Android"
        } else if cfg!(target_os = "ios") {
            "iOS"
        } else {
            "Linux"
        };

        let double_stack_val = if self.double_stack { "1" } else { "0" };

        let url = format!("{}/cgi-bin/srun_portal", self.base_url);
        let password_param = format!("{{MD5}}{}", hmd5);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("action", "login"),
                ("username", self.username.as_str()),
                ("password", password_param.as_str()),
                ("ac_id", self.ac_id.as_str()),
                ("ip", final_ip.as_str()),
                ("info", info_param.as_str()),
                ("chksum", chksum.as_str()),
                ("n", n),
                ("type", auth_type),
                ("os", os_name),
                ("name", os_name),
                ("double_stack", double_stack_val),
                ("nas_ip", nas_ip),
                ("callback", "jQuery123"),
            ])
            .send()
            .map_err(|e| e.to_string())?
            .text()
            .map_err(|e| e.to_string())?;

        let json = Self::extract_jsonp(&resp)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn logout(&self) -> Result<AuthResponse, String> {
        let user_info = self.check_info("")?;

        let ip = user_info["online_ip"]
            .as_str()
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
        let resp_dm_str = self
            .client
            .get(&url_dm)
            .query(&[
                ("username", user_name_only),
                ("ip", ip),
                ("unbind", unbind),
                ("time", time.as_str()),
                ("sign", sign.as_str()),
                ("callback", "jQuery123"),
            ])
            .send()
            .map_err(|e| e.to_string())?
            .text()
            .map_err(|e| e.to_string())?;

        // 步骤 2: 标准 Session 注销 (Logout Normal) - 作为补充调用
        let url_logout = format!("{}/cgi-bin/srun_portal", self.base_url);
        let _ = self
            .client
            .get(&url_logout)
            .query(&[
                ("action", "logout"),
                ("username", username_with_domain.as_str()),
                ("ip", ip),
                ("ac_id", self.ac_id.as_str()),
                ("double_stack", "0"),
                ("callback", "jQuery123"),
            ])
            .send();

        let json = Self::extract_jsonp(&resp_dm_str)?;
        serde_json::from_value(json).map_err(|e| e.to_string())
    }

    pub fn check_info(&self, ip: &str) -> Result<Value, String> {
        let url = format!("{}/cgi-bin/rad_user_info", self.base_url);

        let mut params: Vec<(&str, &str)> = vec![("callback", "jQuery123")];
        if !ip.is_empty() {
            params.push(("ip", ip));
        }

        let resp = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .map_err(|e| e.to_string())?
            .text()
            .map_err(|e| e.to_string())?;

        Self::extract_jsonp(&resp)
    }
}
