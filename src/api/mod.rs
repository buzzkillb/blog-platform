pub mod auth;
pub mod sites;
pub mod posts;
pub mod pages;
pub mod media;

use axum::{Router, routing::{get, post}};

pub fn routes() -> Router<crate::AppState> {
    Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        
        .route("/api/sites", get(sites::list).post(sites::create))
        .route("/api/sites/:id", get(sites::get).put(sites::update).delete(sites::delete))
        
        .route("/api/sites/:site_id/posts", get(posts::list).post(posts::create))
        .route("/api/sites/:site_id/posts/:id", get(posts::get).put(posts::update).delete(posts::delete))
        .route("/api/sites/:site_id/posts/:id/publish", post(posts::publish))
        
        .route("/api/sites/:site_id/pages", get(pages::list).post(pages::create))
        .route("/api/sites/:site_id/pages/:id", get(pages::get).put(pages::update).delete(pages::delete))
        
        .route("/api/sites/:site_id/media", get(media::list).post(media::upload))
        .route("/api/sites/:site_id/media/:id", get(media::get).delete(media::delete))
        
        .route("/api/sites/:site_id/contact", post(sites::submit_contact))
        .route("/api/sites/:site_id/contact", get(sites::list_contact_submissions))
}
