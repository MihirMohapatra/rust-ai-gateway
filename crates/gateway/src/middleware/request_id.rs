use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Adds a unique X-Request-ID header to every request/response for distributed tracing.
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    // Add to tracing span
    let span = tracing::info_span!("request", request_id = %request_id);
    let _guard = span.enter();

    let mut response = next.run(request).await;

    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    response
}

