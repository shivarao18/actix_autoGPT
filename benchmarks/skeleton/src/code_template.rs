// CODE TEMPLATE
// Bind 127.0.0.1, read the PORT environment variable (default 8080), and serve GET /health.
// Interactive runs start from web_template instead of this hermetic skeleton.
fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    println!("template listening on 127.0.0.1:{port}");
}
