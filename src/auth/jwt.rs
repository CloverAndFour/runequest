//! JWT token creation and validation.

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::{RunequestError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new(data_dir: &std::path::Path) -> Result<Self> {
        let key_path = data_dir.join("jwt_secret.key");
        let secret = if key_path.exists() {
            std::fs::read(&key_path)?
        } else {
            let mut secret = vec![0u8; 64];
            rand::thread_rng().fill_bytes(&mut secret);
            std::fs::create_dir_all(data_dir)?;
            std::fs::write(&key_path, &secret)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                let _ = std::fs::set_permissions(&key_path, perms);
            }

            secret
        };

        Ok(Self {
            encoding_key: EncodingKey::from_secret(&secret),
            decoding_key: DecodingKey::from_secret(&secret),
        })
    }

    pub fn create_token(&self, username: &str, role: &str) -> Result<String> {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: username.to_string(),
            role: role.to_string(),
            exp: now + 86400,
            iat: now,
        };
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| RunequestError::InvalidToken(e.to_string()))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| RunequestError::InvalidToken(e.to_string()))?;
        Ok(data.claims)
    }
}
