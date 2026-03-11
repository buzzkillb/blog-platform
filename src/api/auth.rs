use axum::{
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

use crate::{AppState, ApiError, User, CreateUserRequest, LoginRequest};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(ApiError::new("Email and password are required"));
    }

    if payload.password.len() < 8 {
        return Err(ApiError::new("Password must be at least 8 characters"));
    }

    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|e| ApiError::new(format!("Failed to hash password: {}", e)))?;

    let result = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id, email, password_hash, name"
    )
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(&payload.name)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, email, _, name)) => {
            let user = User {
                id,
                email: email.clone(),
                name: name.clone(),
                created_at: chrono::Utc::now(),
            };
            let email_clone = email.clone();
            let name_clone = name.clone();
            
            // Auto-add user to first site if exists
            if let Ok(Some(site_id)) = sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT id FROM sites LIMIT 1"
            )
            .fetch_optional(&state.db)
            .await
            {
                sqlx::query(
                    "INSERT INTO site_members (site_id, user_id, role) VALUES ($1, $2, 'admin') ON CONFLICT DO NOTHING"
                )
                .bind(site_id)
                .bind(id)
                .execute(&state.db)
                .await
                .ok();
                
                return Ok((StatusCode::CREATED, Json(crate::LoginResponse { 
                    user: crate::errors::UserResponse { 
                        id, 
                        email: email_clone, 
                        name: name_clone 
                    }, 
                    site_id,
                    token: "".to_string() 
                })));
            }
            
            Ok((StatusCode::CREATED, Json(crate::LoginResponse { 
                user: crate::errors::UserResponse { 
                    id, 
                    email: email_clone, 
                    name: name_clone 
                }, 
                site_id: None,
                token: "".to_string() 
            })))
        }
        Err(e) => {
            if e.to_string().contains("duplicate") {
                Err(ApiError::new("Email already exists"))
            } else {
                Err(ApiError::new(format!("Failed to create user: {}", e)))
            }
        }
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = sqlx::query_as::<_, (Uuid, String, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, email, password_hash, name, created_at FROM users WHERE email = $1"
    )
    .bind(&payload.email)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Invalid email or password"))?;

    let valid = verify(&payload.password, &user.2)
        .map_err(|_| ApiError::new("Invalid email or password"))?;

    if !valid {
        return Err(ApiError::new("Invalid email or password"));
    }

    let user = User {
        id: user.0,
        email: user.1,
        name: user.3,
        created_at: user.4,
    };

    let site_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT site_id FROM site_members WHERE user_id = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .ok()
    .flatten();

    Ok(Json(crate::LoginResponse { 
        user: crate::errors::UserResponse { 
            id: user.id, 
            email: user.email, 
            name: user.name 
        }, 
        site_id,
        token: "".to_string() 
    }))
}

pub async fn logout() -> impl IntoResponse {
    (StatusCode::OK, "Logged out")
}
