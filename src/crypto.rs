use md5::{Md5, Digest};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use hex;

type HmacMd5 = Hmac<Md5>;

pub fn hmac_md5(key: &str, message: &str) -> String {
    let mut mac = HmacMd5::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

pub fn sha1(data: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn s(data: &[u8], include_len: bool) -> Vec<u32> {
    let len = data.len();
    let mut v = Vec::with_capacity((len + 3) / 4 + if include_len { 1 } else { 0 });
    for i in (0..len).step_by(4) {
        let mut u: u32 = 0;
        for j in 0..4 {
            if i + j < len {
                u |= (data[i + j] as u32) << (j * 8);
            }
        }
        v.push(u);
    }
    if include_len {
        v.push(len as u32);
    }
    v
}

fn l(v: &[u32], include_len: bool) -> Vec<u8> {
    let n = v.len();
    let mut m = (n as usize) << 2;
    if include_len {
        let m_orig = v[n - 1] as usize;
        if m_orig > m {
            return Vec::new(); // Error
        }
        m = m_orig;
    }
    let mut res = Vec::with_capacity(m);
    for i in 0..n {
        let u = v[i];
        for j in 0..4 {
            let byte = ((u >> (j * 8)) & 0xff) as u8;
            if res.len() < m {
                res.push(byte);
            }
        }
    }
    res
}

pub fn xxtea_encode(data: &str, key: &str) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut v = s(data.as_bytes(), true);
    let mut k = s(key.as_bytes(), false);
    if k.len() < 4 {
        k.resize(4, 0);
    }

    let n = (v.len() - 1) as i32;
    let mut z = v[n as usize];
    let mut y;
    let c: u32 = 0x9e3779b9; // 0x86014019 | 0x183639A0
    let mut d: u32 = 0;
    let mut q = 6 + 52 / (v.len() as u32);

    while q > 0 {
        d = d.wrapping_add(c);
        let e = (d >> 2) & 3;
        for p in 0..n {
            y = v[(p + 1) as usize];
            let m = (z >> 5 ^ y << 2)
                .wrapping_add((y >> 3 ^ z << 4) ^ (d ^ y))
                .wrapping_add(k[(p as u32 & 3 ^ e) as usize] ^ z);
            v[p as usize] = v[p as usize].wrapping_add(m);
            z = v[p as usize];
        }
        y = v[0];
        let m = (z >> 5 ^ y << 2)
            .wrapping_add((y >> 3 ^ z << 4) ^ (d ^ y))
            .wrapping_add(k[(n as u32 & 3 ^ e) as usize] ^ z);
        v[n as usize] = v[n as usize].wrapping_add(m);
        z = v[n as usize];
        q -= 1;
    }

    l(&v, false)
}

const CUSTOM_BASE64_ALPHABET: &[u8] = b"LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA";

pub fn custom_base64_encode(data: &[u8]) -> String {
    let mut res = String::new();
    let mut i = 0;
    let len = data.len();
    while i < len {
        let b0 = data[i];
        let b1 = if i + 1 < len { data[i + 1] } else { 0 };
        let b2 = if i + 2 < len { data[i + 2] } else { 0 };

        res.push(CUSTOM_BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        res.push(CUSTOM_BASE64_ALPHABET[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        
        if i + 1 < len {
            res.push(CUSTOM_BASE64_ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            res.push('=');
        }

        if i + 2 < len {
            res.push(CUSTOM_BASE64_ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            res.push('=');
        }
        i += 3;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_md5() {
        let key = "token";
        let message = "password";
        let res = hmac_md5(key, message);
        assert_eq!(res.len(), 32);
    }

    #[test]
    fn test_sha1() {
        let data = "test";
        let res = sha1(data);
        assert_eq!(res, "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
    }

    #[test]
    fn test_xxtea_and_base64_logic() {
        // 模拟一个简单的加密流程
        let info = r#"{"username":"test"}"#;
        let token = "1234567890";
        let encrypted = xxtea_encode(info, token);
        let encoded = custom_base64_encode(&encrypted);
        assert!(!encoded.is_empty());
    }
}
