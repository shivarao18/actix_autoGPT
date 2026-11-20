# 🛠️ Setup Guide

## Quick Setup (5 minutes)

### 1. Clone the Repository

```bash
git clone <your-repo-url>
cd actix_AUTOGPT
```

### 2. Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:
```bash
rustc --version
cargo --version
```

### 3. Create `.env` File

Create a `.env` file in the project root with the following content:

```env
# OpenAI API Configuration
# Get your API key from: https://platform.openai.com/api-keys
OPEN_AI_KEY=sk-your-api-key-here
OPEN_AI_ORG=org-your-organization-id-here

# Model Configuration
OPENAI_MODEL=gpt-4
OPENAI_TEMPERATURE=0.1

# Path configuration. These default to the in-repo web_template.
# CODE_TEMPLATE_PATH=web_template/src/code_template.rs
# WEB_SERVER_PROJECT_PATH=web_template
# EXEC_MAIN_PATH=web_template/src/main.rs
# API_SCHEMA_PATH=schemas/api_schema.json

# Agent Configuration
MAX_BUG_FIXES=3
BUILD_TIMEOUT_SECS=300
ENDPOINT_TIMEOUT_SECS=5

# Logging Configuration
RUST_LOG=info  # Options: trace, debug, info, warn, error
LOG_FORMAT=pretty  # Options: pretty, json
```

### 4. Get OpenAI API Key

1. Visit https://platform.openai.com/api-keys
2. Sign up or log in
3. Click "Create new secret key"
4. Copy the key and add it to your `.env` file
5. Find your organization ID at https://platform.openai.com/account/org-settings
6. Add it to your `.env` file

### 5. Paths

The backend agent reads and writes `web_template` in this repo. Override that with `WEB_SERVER_PROJECT_PATH`, `CODE_TEMPLATE_PATH`, and `EXEC_MAIN_PATH` if you want a different Actix project. You do not need to edit `src/helpers/general.rs`.

### 6. Build the Project

```bash
cargo build
```

This will download all dependencies and compile the project.

### 7. Run Tests (Optional but Recommended)

```bash
cargo test
```

`cargo test` does not call OpenAI. The live tests are ignored; run them with `cargo test -- --ignored` after exporting a key. Replay and the judge run from fixtures with no key.

### 8. Run the Application

```bash
cargo run
```

You should see:
```
What website are we building today?
>
```

Type your request, for example:
```
> I need a REST API that tracks weather data with temperature and descriptions
```

---

## 🚨 Troubleshooting

### Issue: "OPEN_AI_KEY not found in environment variables"

**Solution**: 
- Ensure `.env` file exists in project root
- Check that `OPEN_AI_KEY` is set in `.env`
- Restart your terminal or reload environment

### Issue: "Failed to read code template"

**Solution**:
- Run from the repository root so `web_template` resolves
- Or set `WEB_SERVER_PROJECT_PATH` and `CODE_TEMPLATE_PATH` to a project that contains `src/code_template.rs`

### Issue: Compilation errors

**Solution**:
```bash
# Clean and rebuild
cargo clean
cargo build
```

### Issue: Tests failing

**Solution**:
- Ensure OpenAI API key is valid and has credits
- Some tests require network access
- Check `RUST_LOG=debug cargo test -- --nocapture` for details

### Issue: "Too many requests" from OpenAI

**Solution**:
- You may have hit rate limits
- Wait a few minutes before trying again
- Consider upgrading your OpenAI plan

---

## 📂 Project Structure Setup

### Creating a Web Template Project

If you don't have a `web_template` directory, create one:

```bash
# Create a new Actix-web project
cargo new web_template
cd web_template

# Add Actix-web dependency
cargo add actix-web
```

Create `src/code_template.rs`:
```rust
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct TemplateResponse {
    message: String,
}

async fn index() -> impl Responder {
    HttpResponse::Ok().json(TemplateResponse {
        message: "Template endpoint".to_string(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

---

## 🔧 Development Setup

### Recommended VS Code Extensions

- **rust-analyzer** - Rust language support
- **Better TOML** - TOML syntax highlighting
- **Error Lens** - Inline error display
- **CodeLLDB** - Debugging support

### Recommended Settings

Add to `.vscode/settings.json`:
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true
}
```

### Setting up Pre-commit Hooks

```bash
# Install pre-commit (if using)
pip install pre-commit

# Or use cargo-husky
cargo install cargo-husky
```

Create `.pre-commit-config.yaml`:
```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt
        language: system
        types: [rust]
        pass_filenames: false
        
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
```

---

## 🎯 Verification Checklist

After setup, verify everything works:

- [ ] Rust is installed: `rustc --version`
- [ ] Project compiles: `cargo build`
- [ ] Tests pass: `cargo test`
- [ ] Clippy is happy: `cargo clippy`
- [ ] Formatting is correct: `cargo fmt --check`
- [ ] Application runs: `cargo run`
- [ ] Can make OpenAI API calls (run a test)
- [ ] Logs appear correctly

---

## 🚀 Next Steps

Once setup is complete:

1. **Read the README.md** - Understand the architecture
2. **Run a test generation** - Try generating a simple API
3. **Review the code** - Familiarize yourself with the structure
4. **Implement improvements** - See QUICK_START_IMPROVEMENTS.md
5. **Practice explaining it** - Prepare for interviews

---

## 📚 Additional Resources

### Learning Resources
- [The Rust Book](https://doc.rust-lang.org/book/) - Official Rust guide
- [Async Rust](https://rust-lang.github.io/async-book/) - Async programming
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Async runtime
- [Actix-web Docs](https://actix.rs/docs/) - Web framework

### Community
- [Rust Discord](https://discord.gg/rust-lang) - Get help
- [r/rust](https://reddit.com/r/rust) - Discussions
- [This Week in Rust](https://this-week-in-rust.org/) - News

### Tools
- [crates.io](https://crates.io/) - Package registry
- [docs.rs](https://docs.rs/) - Documentation
- [Rust Playground](https://play.rust-lang.org/) - Online REPL

---

## 🆘 Getting Help

If you encounter issues:

1. **Check the error message** - Rust errors are usually very helpful
2. **Search existing issues** - Someone may have had the same problem
3. **Check dependencies** - Run `cargo update`
4. **Ask for help** - Include error messages and what you've tried
5. **Stack Overflow** - Tag with `rust`, `tokio`, `actix-web`

---

## 📝 Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `OPEN_AI_KEY` | Yes | - | OpenAI API key |
| `OPEN_AI_ORG` | No | - | OpenAI organization ID. Sent only when set. |
| `OPENAI_MODEL` | No | `gpt-4` | Model to use |
| `OPENAI_TEMPERATURE` | No | `0.1` | Response creativity (0-2) |
| `RUST_LOG` | No | `info` | Log level |
| `LOG_FORMAT` | No | `pretty` | Log format (pretty/json) |
| `MAX_BUG_FIXES` | No | `3` | Max automatic fix attempts |
| `BUILD_TIMEOUT_SECS` | No | `300` | Build timeout in seconds |
| `ENDPOINT_TIMEOUT_SECS` | No | `5` | API timeout in seconds |

---

## 🔒 Security Notes

- **Never commit `.env` file** - It's in `.gitignore` for a reason
- **Never share your API key** - It can be used to charge your account
- **Use environment variables** - Don't hardcode secrets
- **Rotate keys regularly** - Especially if exposed
- **Monitor usage** - Check OpenAI dashboard for unexpected charges

---

## ✅ Ready to Go!

Once you've completed setup, you're ready to:
- Generate web applications from natural language
- Explore the codebase
- Implement improvements
- Showcase on your resume

**Happy coding! 🦀**

