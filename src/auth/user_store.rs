//! User storage with argon2id password hashing and API key management.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{RunequestError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

/// A stored API key record. The actual key is never stored -- only its SHA-256 hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    #[serde(default)]
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_login: Option<DateTime<Utc>>,
    #[serde(default)]
    pub api_keys: Vec<ApiKeyRecord>,
}

pub struct UserStore {
    file_path: PathBuf,
}

impl UserStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        Self {
            file_path: data_dir.join("users.json"),
        }
    }

    fn load(&self) -> Result<HashMap<String, User>> {
        if !self.file_path.exists() {
            return Ok(HashMap::new());
        }
        let data = std::fs::read_to_string(&self.file_path)?;
        let users: HashMap<String, User> = serde_json::from_str(&data)?;
        Ok(users)
    }

    fn save(&self, users: &HashMap<String, User>) -> Result<()> {
        use fs2::FileExt;
        use std::io::Write;

        let dir = self.file_path.parent().unwrap();
        std::fs::create_dir_all(dir)?;

        let tmp_path = self.file_path.with_extension("json.tmp");
        let mut tmp = std::fs::File::create(&tmp_path)?;
        tmp.lock_exclusive()?;

        let json = serde_json::to_string_pretty(users)?;
        tmp.write_all(json.as_bytes())?;
        tmp.flush()?;
        tmp.unlock()?;

        std::fs::rename(&tmp_path, &self.file_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&self.file_path, perms);
        }

        Ok(())
    }

    pub fn has_users(&self) -> bool {
        self.load().map(|u| !u.is_empty()).unwrap_or(false)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        let users = self.load()?;
        let mut list: Vec<User> = users.into_values().collect();
        list.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(list)
    }

    pub fn create_user(&self, username: &str, password: &str, role: UserRole) -> Result<()> {
        validate_username(username)?;

        let mut users = self.load()?;
        if users.contains_key(username) {
            return Err(RunequestError::UserAlreadyExists(username.to_string()));
        }

        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| RunequestError::AuthenticationFailed)?
            .to_string();

        let user = User {
            username: username.to_string(),
            password_hash: hash,
            role,
            created_at: Utc::now(),
            last_login: None,
            api_keys: Vec::new(),
        };

        users.insert(username.to_string(), user);
        self.save(&users)?;
        Ok(())
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<User> {
        let users = self.load()?;
        let user = users
            .get(username)
            .ok_or_else(|| RunequestError::UserNotFound(username.to_string()))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| RunequestError::AuthenticationFailed)?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| {
                std::thread::sleep(std::time::Duration::from_secs(1));
                RunequestError::AuthenticationFailed
            })?;

        // Update last_login
        let mut users = self.load()?;
        if let Some(u) = users.get_mut(username) {
            u.last_login = Some(Utc::now());
            let _ = self.save(&users);
        }

        Ok(user.clone())
    }

    /// Change a user's password. Verifies current password first.
    pub fn change_password(
        &self,
        username: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        if new_password.len() < 8 {
            return Err(RunequestError::InvalidGameState(
                "New password must be at least 8 characters".to_string(),
            ));
        }

        // Verify current password
        let users = self.load()?;
        let user = users
            .get(username)
            .ok_or_else(|| RunequestError::UserNotFound(username.to_string()))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| RunequestError::AuthenticationFailed)?;

        Argon2::default()
            .verify_password(current_password.as_bytes(), &parsed_hash)
            .map_err(|_| {
                std::thread::sleep(std::time::Duration::from_secs(1));
                RunequestError::AuthenticationFailed
            })?;

        // Hash new password and save
        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|_| RunequestError::AuthenticationFailed)?
            .to_string();

        let mut users = self.load()?;
        if let Some(u) = users.get_mut(username) {
            u.password_hash = new_hash;
            self.save(&users)?;
        }
        Ok(())
    }

    /// Create a new API key for a user. Returns the plaintext key (shown once).
    pub fn create_api_key(&self, username: &str, name: &str) -> Result<(ApiKeyRecord, String)> {
        if name.is_empty() || name.len() > 64 {
            return Err(RunequestError::InvalidGameState(
                "API key name must be 1-64 characters".to_string(),
            ));
        }

        let mut users = self.load()?;
        let user = users
            .get_mut(username)
            .ok_or_else(|| RunequestError::UserNotFound(username.to_string()))?;

        if user.api_keys.len() >= 10 {
            return Err(RunequestError::InvalidGameState(
                "Maximum 10 API keys per user".to_string(),
            ));
        }

        // Generate key: rq_ + 32 hex chars (128 bits)
        let mut key_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let plaintext_key = format!("rq_{}", hex::encode(key_bytes));

        // Hash it
        let mut hasher = Sha256::new();
        hasher.update(plaintext_key.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let record = ApiKeyRecord {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            name: name.to_string(),
            key_hash,
            prefix: plaintext_key[..12].to_string(),
            created_at: Utc::now(),
            last_used: None,
        };

        user.api_keys.push(record.clone());
        self.save(&users)?;

        Ok((record, plaintext_key))
    }

    /// List API keys for a user (without hashes).
    pub fn list_api_keys(&self, username: &str) -> Result<Vec<ApiKeyRecord>> {
        let users = self.load()?;
        let user = users
            .get(username)
            .ok_or_else(|| RunequestError::UserNotFound(username.to_string()))?;
        Ok(user.api_keys.clone())
    }

    /// Revoke (delete) an API key by its ID.
    pub fn revoke_api_key(&self, username: &str, key_id: &str) -> Result<()> {
        let mut users = self.load()?;
        let user = users
            .get_mut(username)
            .ok_or_else(|| RunequestError::UserNotFound(username.to_string()))?;

        let before = user.api_keys.len();
        user.api_keys.retain(|k| k.id != key_id);
        if user.api_keys.len() == before {
            return Err(RunequestError::InvalidGameState(
                "API key not found".to_string(),
            ));
        }
        self.save(&users)?;
        Ok(())
    }

    /// Authenticate an API key. Returns the user if valid.
    pub fn authenticate_api_key(&self, key: &str) -> Result<User> {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let mut users = self.load()?;
        for user in users.values_mut() {
            for api_key in &mut user.api_keys {
                if api_key.key_hash == key_hash {
                    api_key.last_used = Some(Utc::now());
                    let result = user.clone();
                    self.save(&users)?;
                    return Ok(result);
                }
            }
        }
        Err(RunequestError::AuthenticationFailed)
    }
}

fn validate_username(username: &str) -> Result<()> {
    if username.len() < 3 || username.len() > 32 {
        return Err(RunequestError::InvalidUsername(
            "username must be 3-32 characters".to_string(),
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(RunequestError::InvalidUsername(
            "username must be lowercase alphanumeric or hyphens".to_string(),
        ));
    }
    if username.starts_with('-') || username.ends_with('-') {
        return Err(RunequestError::InvalidUsername(
            "username must not start or end with a hyphen".to_string(),
        ));
    }
    Ok(())
}
