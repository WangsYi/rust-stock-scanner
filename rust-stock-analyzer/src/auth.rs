use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use bcrypt::{hash, verify};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{Duration, Utc};

use crate::models::{User, LoginRequest, RegisterRequest, AuthResponse, UserResponse, AuthConfig};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    username: String,
    exp: usize,
    is_admin: bool,
}

pub struct AuthService {
    users: Arc<Mutex<HashMap<String, User>>>,
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        let mut users = HashMap::new();
        
        // Add default admin user if no users exist
        if config.enabled {
            let admin = User {
                id: Uuid::new_v4().to_string(),
                username: "admin".to_string(),
                email: "admin@example.com".to_string(),
                password_hash: hash("admin123", config.bcrypt_cost).unwrap_or_default(),
                created_at: Utc::now(),
                last_login: None,
                is_admin: true,
                api_usage: 0,
                is_active: true,
            };
            users.insert(admin.username.clone(), admin);
        }

        Self {
            users: Arc::new(Mutex::new(users)),
            config,
        }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse, String> {
        if !self.config.enabled {
            return Err("Authentication is disabled".to_string());
        }

        let mut users = self.users.lock().await;
        
        if users.contains_key(&req.username) {
            return Err("Username already exists".to_string());
        }

        let password_hash = hash(&req.password, self.config.bcrypt_cost)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        let user = User {
            id: Uuid::new_v4().to_string(),
            username: req.username.clone(),
            email: req.email,
            password_hash,
            created_at: Utc::now(),
            last_login: None,
            is_admin: false,
            api_usage: 0,
            is_active: true,
        };

        let token = self.generate_token(&user)?;
        let user_response = self.user_to_response(&user);

        users.insert(req.username, user);

        Ok(AuthResponse {
            token,
            user: user_response,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, String> {
        if !self.config.enabled {
            return Err("Authentication is disabled".to_string());
        }

        let mut users = self.users.lock().await;
        
        let user = users.get_mut(&req.username)
            .ok_or_else(|| "User not found".to_string())?;

        if !user.is_active {
            return Err("Account is disabled".to_string());
        }

        if !verify(&req.password, &user.password_hash)
            .map_err(|_| "Invalid password".to_string())? {
            return Err("Invalid credentials".to_string());
        }

        user.last_login = Some(Utc::now());
        user.api_usage += 1;

        let token = self.generate_token(user)?;
        let user_response = self.user_to_response(user);

        Ok(AuthResponse {
            token,
            user: user_response,
        })
    }

    pub fn generate_token(&self, user: &User) -> Result<String, String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::seconds(self.config.session_timeout as i64))
            .ok_or("Invalid expiration time")?;

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            exp: expiration.timestamp() as usize,
            is_admin: user.is_admin,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.secret_key.as_bytes()),
        )
        .map_err(|e| format!("Failed to generate token: {}", e))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.secret_key.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| format!("Invalid token: {}", e))?;

        Ok(token_data.claims)
    }

    pub async fn get_user(&self, username: &str) -> Option<User> {
        let users = self.users.lock().await;
        users.get(username).cloned()
    }

    pub async fn increment_api_usage(&self, username: &str) -> Result<(), String> {
        let mut users = self.users.lock().await;
        if let Some(user) = users.get_mut(username) {
            user.api_usage += 1;
        }
        Ok(())
    }

    fn user_to_response(&self, user: &User) -> UserResponse {
        UserResponse {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            is_admin: user.is_admin,
            api_usage: user.api_usage,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn get_config(&self) -> &AuthConfig {
        &self.config
    }
}