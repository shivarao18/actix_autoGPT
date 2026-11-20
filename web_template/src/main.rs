use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct Health {
    status: String,
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(Health {
        status: "ok".to_string(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{port}");
    println!("listening on {addr}");
    HttpServer::new(|| App::new().service(health))
        .bind(addr)?
        .run()
        .await
}
