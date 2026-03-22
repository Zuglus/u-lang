use axum::{Router, extract::Request, response::IntoResponse, http::StatusCode};

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") { "text/html" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".js") { "application/javascript" }
    else if path.ends_with(".json") { "application/json" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
    else { "application/octet-stream" }
}

async fn handler(req: Request) -> impl IntoResponse {
    let mut path = req.uri().path().to_string();
    if path == "/" { path = "/index.html".to_string(); }
    let filepath = format!("public{}", path);
    match std::fs::read_to_string(&filepath) {
        Ok(content) => (StatusCode::OK, [("content-type", mime_type(&filepath))], content),
        Err(_) => (StatusCode::NOT_FOUND, [("content-type", "text/plain")], "404 Not Found".to_string()),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().fallback(handler);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Serving static files from ./public on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
