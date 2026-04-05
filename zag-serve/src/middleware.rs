//! Request logging middleware for the zag server.

use axum::{extract::Request, middleware::Next, response::Response};

pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    log::debug!("[→] {} {}", method, path);
    let response = next.run(request).await;
    log::debug!("[←] {} {} → {}", method, path, response.status());
    response
}
