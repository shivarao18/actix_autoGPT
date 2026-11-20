use ai_functions::ai_function;

#[ai_function]
pub fn print_backend_webserver_code(_project_description_and_template: &str) {
    /// INPUT: Takes in a PROJECT_DESCRIPTION and CODE_TEMPLATE for a website backend build
    /// FUNCTION: Takes an existing set of code marked as CODE_TEMPLATE and updates it based on the PROJECT_DESCRIPTION
    /// IMPORTANT: The following crates are already installed. Do not use any other crates.
    ///   actix-web, actix-cors, reqwest, serde, serde_json, tokio
    /// The server must bind 127.0.0.1 and read the port from the PORT environment variable, defaulting to 8080.
    /// Include a GET /health route that returns HTTP 200 and JSON {"status":"ok"}.
    /// OUTPUT: Prints out the CODE only. Do not comment on the code. Print the CODE only.
    println!(OUTPUT)
}

#[ai_function]
pub fn print_improved_webserver_code(_project_description_and_template: &str) {
    /// INPUT: Takes in a PROJECT_DESCRIPTION and CODE for a website backend build
    /// FUNCTION: Performs the following tasks:
    ///   1. Removes any comments from the code
    ///   2. Implements all user requested logic from PROJECT_DESCRIPTION
    ///   3. Implements proper error handling
    ///   4. Keeps a GET /health route that returns HTTP 200
    /// IMPORTANT: The following crates are already installed. Do not use any other crates.
    ///   actix-web, actix-cors, reqwest, serde, serde_json, tokio
    /// Keep reading the listen port from the PORT environment variable, default 8080.
    /// OUTPUT: Prints out the CODE only. Do not comment on the code. Print the CODE only.
    println!(OUTPUT)
}

#[ai_function]
pub fn print_fixed_code(_broken_code_with_bugs: &str) {
    /// INPUT: Takes in Rust BROKEN_CODE and ERROR_BUGS found during compilation
    /// FUNCTION: Removes bugs from code only. Do not add features.
    /// IMPORTANT: The following crates are already installed. Do not use any other crates.
    ///   actix-web, actix-cors, reqwest, serde, serde_json, tokio
    /// Keep the PORT environment variable and the GET /health route.
    /// OUTPUT: Prints out the CODE only. Do not comment on the code. Print the CODE only.
    println!(OUTPUT)
}

#[ai_function]
pub fn print_rest_api_endpoints(_code_input: &str) {
    /// INPUT: Takes in Rust webserver CODE_INPUT based on actix-web or a small HTTP server
    /// FUNCTION: Prints out the JSON schema for url endpoints and their respective types
    /// LOGIC:
    ///   Extract every route the server handles
    /// IMPORTANT: Only print a JSON array. No markdown fences.
    /// method must be lowercase "get" or "post".
    /// is_route_dynamic must be the string "true" or "false".
    /// OUTPUT: Prints out a JSON array of the following format:
    /// [
    ///   {
    ///     "route": "/path",
    ///     "is_route_dynamic": "false",
    ///     "method": "get",
    ///     "request_body": "None",
    ///     "response": {"status": "string"}
    ///   }
    /// ]
    println!(OUTPUT)
}
