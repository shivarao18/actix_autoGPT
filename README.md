# 🤖 Rust AutoGPT - Autonomous Code Generation System

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![OpenAI](https://img.shields.io/badge/OpenAI-412991.svg?style=for-the-badge&logo=OpenAI&logoColor=white)
![Actix](https://img.shields.io/badge/actix-web-blue?style=for-the-badge)

An intelligent, autonomous Rust application that leverages OpenAI's GPT-4 API to automatically generate, architect, test, and deploy full-stack web applications from natural language descriptions.

## 🎯 Project Overview

This project implements a sophisticated multi-agent system in Rust that can:
- **Understand user requirements** through natural language processing
- **Design system architecture** with automatic technology stack selection
- **Generate production-ready backend code** using Actix-web framework
- **Validate external API endpoints** with automated health checks
- **Test and debug generated code** with automated build verification
- **Deploy working applications** with minimal human intervention

## 🏗️ Architecture

### Multi-Agent System Design

The system employs a hierarchical agent architecture inspired by software development teams:

```
┌─────────────────────────────────────────┐
│         Managing Agent                  │
│  (Project Manager - Orchestration)      │
└────────────┬────────────────────────────┘
             │
             ├──────────────────┬─────────────────┐
             ▼                  ▼                 ▼
    ┌────────────────┐  ┌─────────────┐  ┌──────────────┐
    │   Solution     │  │   Backend   │  │   Future     │
    │   Architect    │  │  Developer  │  │   Agents     │
    │                │  │             │  │   (TBD)      │
    └────────────────┘  └─────────────┘  └──────────────┘
```

### Core Components

#### 1. **Managing Agent** (`ManagingAgent`)
- **Role**: Project Manager and Orchestrator
- **Responsibilities**:
  - Converts user input into structured project goals
  - Coordinates specialist agents
  - Maintains shared state via `FactSheet`
  - Manages project lifecycle

#### 2. **Solution Architect Agent** (`AgentSolutionArchitect`)
- **Role**: System Designer and API Validator
- **Responsibilities**:
  - Analyzes project requirements
  - Determines feature requirements (CRUD, Auth, External APIs)
  - Identifies and validates external API endpoints
  - Performs health checks on third-party services
  
#### 3. **Backend Developer Agent** (`AgentBackendDeveloper`)
- **Role**: Code Generator and Quality Assurance
- **Responsibilities**:
  - Generates Actix-web backend code
  - Extracts REST API endpoints
  - Builds and tests generated code
  - Automatically fixes compilation errors (up to 3 attempts)
  - Validates running endpoints

### Data Flow

```rust
User Input (Natural Language)
    ↓
Managing Agent: Convert to goal
    ↓
Solution Architect: Analyze requirements
    ↓
Solution Architect: Validate external APIs
    ↓
Backend Developer: Generate initial code
    ↓
Backend Developer: Improve code
    ↓
Backend Developer: Build & Test
    ↓ (if errors)
Backend Developer: Fix bugs (max 3 iterations)
    ↓
Backend Developer: Deploy & verify endpoints
    ↓
Working Application ✓
```

## 🚀 Key Features

### Intelligent Code Generation
- **GPT-4 powered**: Utilizes OpenAI's most advanced language model
- **Context-aware**: Understands project requirements and constraints
- **Template-based**: Uses code templates for consistent output
- **Iterative improvement**: Refines code quality through multiple passes

### Automated Testing & Validation
- **Build verification**: Automatically compiles generated code
- **Runtime testing**: Spawns test servers and validates endpoints
- **API health checks**: Tests external dependencies
- **Self-healing**: Attempts to fix compilation errors automatically

### Type-Safe Design
- **Serde integration**: Robust JSON serialization/deserialization
- **Strong typing**: Leverages Rust's type system for reliability
- **Custom data models**: `RouteObject`, `ProjectScope`, `FactSheet`
- **Error handling**: Comprehensive `Result` types

### Async/Await Architecture
- **Tokio runtime**: High-performance async execution
- **Non-blocking I/O**: Efficient API calls and network operations
- **Concurrent operations**: Parallel agent execution where possible

## 📦 Project Structure

```
actix_AUTOGPT/
├── src/
│   ├── main.rs                 # Entry point, orchestration
│   ├── ai_functions/           # AI function definitions
│   │   ├── aifunc_managing.rs  # Managing agent AI functions
│   │   ├── aifunc_architect.rs # Architect agent AI functions
│   │   └── aifunc_backend.rs   # Backend agent AI functions
│   ├── apis/                   # External API integrations
│   │   └── call_request.rs     # OpenAI API client
│   ├── helpers/                # Utility functions
│   │   ├── command_line.rs     # CLI interactions
│   │   └── general.rs          # Common helpers
│   └── models/                 # Data structures & agents
│       ├── agent_basic/        # Base agent traits
│       ├── agents/             # Specialist agent implementations
│       │   ├── agent_architect.rs
│       │   ├── agent_backend.rs
│       │   └── agent_traits.rs
│       ├── agents_manager/     # Managing agent
│       │   └── managing_agent.rs
│       └── general/            # Shared models
│           └── llm.rs          # LLM data structures
└── schemas/
    └── api_schema.json         # Generated API schemas
```

## 🛠️ Technical Stack

- **Language**: Rust (Edition 2024)
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest
- **Serialization**: Serde (JSON)
- **AI Model**: OpenAI GPT-4
- **Web Framework**: Actix-web (for generated code)
- **Traits**: Async-trait for polymorphic agents

## ⚙️ Setup & Installation

### Prerequisites
- Rust (1.70+)
- OpenAI API Key
- OpenAI Organization ID

### Configuration

1. **Clone the repository**:
```bash
git clone <repository-url>
cd actix_AUTOGPT
```

2. **Set up environment variables**:
Create a `.env` file in the project root:
```env
OPEN_AI_KEY=your_openai_api_key_here
OPEN_AI_ORG=your_openai_org_id_here
```

3. **Update file paths** (in `src/helpers/general.rs`):
```rust
// Update these paths to match your system
const CODE_TEMPLATE_PATH: &str = "/path/to/your/web_template/src/code_template.rs";
pub const WEB_SERVER_PROJECT_PATH: &str = "/path/to/your/web_template/";
const EXEC_MAIN_PATH: &str = "/path/to/your/web_template/src/main.rs";
```

4. **Build the project**:
```bash
cargo build --release
```

## 🎮 Usage

### Basic Usage

Run the application:
```bash
cargo run
```

When prompted, describe the website you want to build:
```
What website are we building today?
> I need a REST API that tracks weather data with temperature and descriptions
```

The system will:
1. ✅ Analyze your requirements
2. ✅ Design the architecture
3. ✅ Generate Actix-web backend code
4. ✅ Test and validate the code
5. ✅ Deploy a working server

### Example Inputs

**E-commerce API**:
```
Build a website that manages products with CRUD operations and integrates with payment APIs
```

**Fitness Tracker**:
```
Need a full stack app that fetches and tracks my fitness progress with timezone information
```

**Crypto Dashboard**:
```
Create a website that fetches and stores crypto price data from Binance
```

## 🧪 Testing

Run the test suite:
```bash
cargo test
```

### Available Tests
- `tests_call_to_openai`: Validates OpenAI API integration
- `tests_managing_agent`: Tests end-to-end agent orchestration
- `tests_solution_architect`: Validates architecture agent
- `tests_backend_developer`: Tests code generation and deployment

## 🔑 Key Design Patterns

### 1. **Trait-Based Polymorphism**
```rust
#[async_trait]
pub trait SpecialFunctions: Debug {
    fn get_attributes_from_agent(&self) -> &BasicAgent;
    async fn execute(&mut self, factsheet: &mut FactSheet) -> Result<(), Box<dyn std::error::Error>>;
}
```

### 2. **State Machine Pattern**
```rust
pub enum AgentState {
    Discovery,
    Working,
    UnitTesting,
    Finished,
}
```

### 3. **Shared State via FactSheet**
```rust
pub struct FactSheet {
    pub project_description: String,
    pub project_scope: Option<ProjectScope>,
    pub external_urls: Option<Vec<String>>,
    pub backend_code: Option<String>,
    pub api_endpoint_schema: Option<Vec<RouteObject>>,
}
```

### 4. **Procedural Macros for AI Functions**
```rust
#[ai_function]
pub fn convert_user_input_to_goal(_user_request: &str) {
    /// Input: Takes in a user request
    /// Function: Converts user request into a short summarized goal
    /// Output: Prints goal
    println!(OUTPUT)
}
```

## 🎓 Learning Outcomes

This project demonstrates proficiency in:
- ✅ **Advanced Rust**: Traits, async/await, lifetimes, error handling
- ✅ **AI Integration**: OpenAI API, prompt engineering, LLM orchestration
- ✅ **System Design**: Multi-agent architectures, state management
- ✅ **API Development**: REST APIs, HTTP clients, endpoint testing
- ✅ **Automation**: Build systems, testing, self-healing code
- ✅ **Type Safety**: Serde, custom types, validated data structures

## 🚧 Future Enhancements

### Planned Features
- [ ] Frontend generation (React/Vue/Svelte)
- [ ] Database schema generation and migration
- [ ] Docker containerization
- [ ] CI/CD pipeline integration
- [ ] Support for multiple LLM providers (Anthropic, Gemini)
- [ ] Incremental code updates instead of full regeneration
- [ ] Advanced error recovery with LLM-based debugging
- [ ] Performance metrics and cost tracking
- [ ] Plugin system for custom agents

### Potential Improvements
- [ ] Add structured logging with `tracing`
- [ ] Implement retry logic with exponential backoff
- [ ] Add configuration management (config files)
- [ ] Create custom error types
- [ ] Add performance benchmarks
- [ ] Implement caching for API responses
- [ ] Add telemetry and observability

## 📊 Performance Characteristics

- **Average project generation time**: 2-5 minutes
- **OpenAI API calls per project**: ~5-10 calls
- **Success rate**: ~85% (depends on complexity)
- **Max bug fix iterations**: 3 attempts
- **Endpoint validation timeout**: 5 seconds

## 🔒 Safety Features

1. **User confirmation**: Requires manual approval before running generated code
2. **Timeout protection**: Network requests have timeouts
3. **Iteration limits**: Prevents infinite loops in bug fixing
4. **Graceful error handling**: Fails safely on critical errors

## 📝 License

[Specify your license here]

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 👨‍💻 Author

Built as part of the Udemy course: *"Learn Rust Whilst Taking ChatGPT to the Extreme In Creating an Automated GPT that Builds and Tests Code for You"*

## 🙏 Acknowledgments

- OpenAI for GPT-4 API
- Rust community for excellent async ecosystem
- Actix-web team for the web framework

---

**Note**: This is an educational project demonstrating AI-powered code generation. Always review and test generated code before deploying to production.

