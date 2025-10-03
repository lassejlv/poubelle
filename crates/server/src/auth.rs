use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct AuthStore {
    users: HashMap<String, String>,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, username: String, password: String) {
        let hash = Self::hash_password(&password);
        self.users.insert(username, hash);
    }

    pub fn verify(&self, username: &str, password: &str) -> bool {
        if let Some(stored_hash) = self.users.get(username) {
            let hash = Self::hash_password(password);
            return &hash == stored_hash;
        }
        false
    }

    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}
