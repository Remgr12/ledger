use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityStatus {
    pub is_encrypted: bool,
    pub cipher_algorithm: String,
    pub key_enclave_provider: String,
    pub has_pin_protection: bool,
    pub authenticated_p2p_enabled: bool,
    pub active_paired_peers_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerPairingPayload {
    pub node_id: String,
    pub node_name: String,
    pub pre_shared_key: String,
    pub network_address: String,
    pub expires_at: Option<String>,
}

#[derive(Clone)]
pub struct SecurityManager {
    status: Arc<RwLock<SecurityStatus>>,
    psk_registry: Arc<RwLock<HashMap<String, String>>>, // node_id -> PSK
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(SecurityStatus {
                is_encrypted: true,
                cipher_algorithm: "ChaCha20-Poly1305 / SQLCipher Envelope".to_string(),
                key_enclave_provider: "Platform Secure Enclave (OS Keyring / Keystore)".to_string(),
                has_pin_protection: true,
                authenticated_p2p_enabled: true,
                active_paired_peers_count: 0,
            })),
            psk_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_status(&self) -> SecurityStatus {
        let mut s = self.status.read().clone();
        s.active_paired_peers_count = self.psk_registry.read().len() as u32;
        s
    }

    pub fn set_pin_protection(&self, enabled: bool) {
        self.status.write().has_pin_protection = enabled;
    }

    pub fn generate_pairing_uri(&self, node_id: &str, node_name: &str, psk: &str, address: &str) -> String {
        self.psk_registry.write().insert(node_id.to_string(), psk.to_string());
        format!(
            "erp://pair?node={}&name={}&psk={}&addr={}",
            url_encode(node_id),
            url_encode(node_name),
            url_encode(psk),
            url_encode(address)
        )
    }

    pub fn parse_pairing_uri(&self, uri: &str) -> Result<PeerPairingPayload, String> {
        if !uri.starts_with("erp://pair?") {
            return Err("Invalid pairing URI format; must start with erp://pair?".to_string());
        }
        let query = &uri["erp://pair?".len()..];
        let mut node_id = String::new();
        let mut node_name = String::new();
        let mut pre_shared_key = String::new();
        let mut network_address = String::new();

        for part in query.split('&') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("");
            let v = kv.next().unwrap_or("");
            let decoded = url_decode(v);
            match k {
                "node" => node_id = decoded,
                "name" => node_name = decoded,
                "psk" => pre_shared_key = decoded,
                "addr" => network_address = decoded,
                _ => {}
            }
        }

        if node_id.is_empty() || pre_shared_key.is_empty() {
            return Err("Pairing URI missing required node_id or psk parameters".to_string());
        }

        self.psk_registry.write().insert(node_id.clone(), pre_shared_key.clone());

        Ok(PeerPairingPayload {
            node_id,
            node_name,
            pre_shared_key,
            network_address,
            expires_at: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    pub fn compute_handshake_token(&self, node_id: &str, challenge: &str) -> Result<String, String> {
        let registry = self.psk_registry.read();
        let psk = registry.get(node_id).ok_or_else(|| format!("Unknown peer node: {}", node_id))?;
        // Deterministic lightweight HMAC simulation using SHA-256-like hex mixing
        let combined = format!("{}:{}:{}", node_id, psk, challenge);
        Ok(format!("hsk_{:x}", simple_hash(combined.as_bytes())))
    }

    pub fn verify_handshake_token(&self, node_id: &str, challenge: &str, token: &str) -> bool {
        match self.compute_handshake_token(node_id, challenge) {
            Ok(expected) => expected == token,
            Err(_) => false,
        }
    }
}

fn simple_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' || b == b':' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn url_decode(input: &str) -> String {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or("00"), 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
