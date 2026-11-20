#[macro_export]
macro_rules! get_function_string {
    ($func: ident) => {{ stringify!($func) }};
}

#[macro_use]
mod ai_functions;
mod apis;
mod cli;
mod helpers;
mod llm;
mod models;
mod replay;
mod run_context;
mod scoring;
mod telemetry;
mod trajectory;
mod verification;

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    let code = async_main().await;
    std::process::exit(code);
}

async fn async_main() -> i32 {
    dotenv().ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let guard = telemetry::init_from_env(cli::traces_file_override(&args));
    let code = match cli::dispatch(&args).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    };
    drop(guard);
    code
}
