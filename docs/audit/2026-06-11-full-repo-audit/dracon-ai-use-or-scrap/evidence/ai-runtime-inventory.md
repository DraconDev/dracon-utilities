# AI runtime crates inventory

## timestamp
2026-06-11T18:53:37+01:00

## manifest references
./AGENTS.md:27:│   ├── ai/dracon-ai-contracts   # RoutingTask, SelectionConstraints
./AGENTS.md:28:│   └── ai/dracon-ai-runtime-contracts # ChatMessage, AiProvider trait
./AGENTS.md:31:    ├── ai-routing-runtime      # SmartRouter for model selection
./AGENTS.md:32:    ├── ai-runtime-config       # Runtime config types
./AGENTS.md:33:    └── ai-runtime-adapters     # GenericOpenAIAdapter for OpenAI-compatible APIs
./README.md:20:│   ├── ai/dracon-ai-contracts     # RoutingTask, SelectionConstraints
./README.md:21:│   └── ai/dracon-ai-runtime-contracts  # ChatMessage, AiProvider trait
./README.md:25:        ├── ai-routing-runtime      # SmartRouter for model selection
./README.md:26:        ├── ai-runtime-config       # Runtime config types
./README.md:27:        └── ai-runtime-adapters     # OpenAI-compatible adapter
./README.md:50:| `dracon-ai-contracts` | `RoutingTask`, `SelectionConstraints`, `ServiceLevel` |
./README.md:51:| `dracon-ai-runtime-contracts` | `ChatMessage`, `ChatRequest`, `AiProvider` trait |
./README.md:58:| `ai-routing-runtime` | `SmartRouter` with model selection |
./README.md:59:| `ai-runtime-config` | `OpenAIProviderSpec` and `AiRuntimeConfig` types |
./README.md:60:| `ai-runtime-adapters` | `GenericOpenAIAdapter` for OpenAI-compatible APIs |
./services/crates/ai/ai-runtime-config/Cargo.toml:2:name = "ai-runtime-config"
./services/crates/ai/ai-runtime-adapters/Cargo.toml:2:name = "ai-runtime-adapters"
./services/crates/ai/ai-runtime-adapters/Cargo.toml:16:dracon-ai-runtime-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-runtime-contracts" }
./services/crates/ai/ai-routing-runtime/Cargo.toml:2:name = "ai-routing-runtime"
./services/crates/ai/ai-routing-runtime/Cargo.toml:14:dracon-ai-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-contracts" }
./services/crates/ai/ai-routing-runtime/Cargo.toml:15:dracon-ai-runtime-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-runtime-contracts" }
./Cargo.toml:14:  "contracts/crates/ai/dracon-ai-contracts",
./Cargo.toml:15:  "contracts/crates/ai/dracon-ai-runtime-contracts",
./Cargo.toml:18:  "services/crates/ai/ai-routing-runtime",
./Cargo.toml:19:  "services/crates/ai/ai-runtime-config",
./Cargo.toml:20:  "services/crates/ai/ai-runtime-adapters",
./contracts/crates/ai/dracon-ai-contracts/Cargo.toml:2:name = "dracon-ai-contracts"
./services/crates/ai/ai-service/Cargo.toml:17:dracon-ai-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-contracts" }
./services/crates/ai/ai-service/Cargo.toml:18:dracon-ai-runtime-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-runtime-contracts" }
./services/crates/ai/ai-service/Cargo.toml:19:ai-routing-runtime = { path = "../ai-routing-runtime" }
./services/crates/ai/ai-service/Cargo.toml:20:ai-runtime-adapters = { path = "../ai-runtime-adapters" }
./contracts/crates/ai/dracon-ai-runtime-contracts/Cargo.toml:2:name = "dracon-ai-runtime-contracts"
./contracts/crates/ai/dracon-ai-runtime-contracts/Cargo.toml:15:dracon-ai-contracts = { path = "../dracon-ai-contracts" }
./Cargo.lock:35:name = "ai-routing-runtime"
./Cargo.lock:40: "dracon-ai-contracts",
./Cargo.lock:41: "dracon-ai-runtime-contracts",
./Cargo.lock:47:name = "ai-runtime-adapters"
./Cargo.lock:52: "dracon-ai-runtime-contracts",
./Cargo.lock:59:name = "ai-runtime-config"
./Cargo.lock:71: "ai-routing-runtime",
./Cargo.lock:72: "ai-runtime-adapters",
./Cargo.lock:75: "dracon-ai-contracts",
./Cargo.lock:76: "dracon-ai-runtime-contracts",
./Cargo.lock:1141:name = "dracon-ai-contracts"
./Cargo.lock:1148:name = "dracon-ai-runtime-contracts"
./Cargo.lock:1153: "dracon-ai-contracts",

## manifest files

===== services/crates/ai/ai-routing-runtime/Cargo.toml =====
[package]
name = "ai-routing-runtime"
version.workspace = true
edition.workspace = true
license = "AGPL-3.0-only"
authors = ["Dracon <dracon@void>"]
repository = "https://github.com/DraconDev/dracon-libs"
description = "AI routing runtime with SmartRouter and ProviderRegistry for model selection"

[dependencies]
serde = { version = "1", features = ["derive"] }
anyhow = "1"
async-trait = "0.1"
dracon-ai-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-contracts" }
dracon-ai-runtime-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-runtime-contracts" }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[lib]
name = "ai_routing_runtime"
path = "src/lib.rs"

===== services/crates/ai/ai-runtime-adapters/Cargo.toml =====
[package]
name = "ai-runtime-adapters"
version.workspace = true
edition.workspace = true
license = "AGPL-3.0-only"
authors = ["Dracon <dracon@void>"]
repository = "https://github.com/DraconDev/dracon-libs"
description = "AI runtime adapters: GenericOpenAIAdapter for OpenAI-compatible API endpoints"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json"] }
dracon-ai-runtime-contracts = { path = "../../../../contracts/crates/ai/dracon-ai-runtime-contracts" }

[lib]
name = "ai_runtime_adapters"
path = "src/lib.rs"

===== services/crates/ai/ai-runtime-config/Cargo.toml =====
[package]
name = "ai-runtime-config"
version.workspace = true
edition.workspace = true
license = "AGPL-3.0-only"
authors = ["Dracon <dracon@void>"]
repository = "https://github.com/DraconDev/dracon-libs"
description = "AI runtime configuration types: OpenAIProviderSpec and AiRuntimeConfig"

[dependencies]
anyhow = { workspace = true }
serde = { version = "1", features = ["derive"] }
serde_json = { workspace = true }

[lib]
name = "ai_runtime_config"
path = "src/lib.rs"

===== contracts/crates/ai/dracon-ai-contracts/Cargo.toml =====
[package]
name = "dracon-ai-contracts"
version.workspace = true
edition.workspace = true
license = "AGPL-3.0-only"
authors = ["Dracon <dracon@void>"]
repository = "https://github.com/DraconDev/dracon-libs"
description = "AI routing contracts: RoutingTask, SelectionConstraints, and ServiceLevel"

[dependencies]
serde = { version = "1", features = ["derive"] }

[lib]
name = "dracon_ai_contracts"
path = "src/lib.rs"

===== contracts/crates/ai/dracon-ai-runtime-contracts/Cargo.toml =====
[package]
name = "dracon-ai-runtime-contracts"
version.workspace = true
edition.workspace = true
license = "AGPL-3.0-only"
authors = ["Dracon <dracon@void>"]
repository = "https://github.com/DraconDev/dracon-libs"
description = "AI runtime contracts: ChatMessage, ChatRequest, and AiProvider trait"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
anyhow = "1"
dracon-ai-contracts = { path = "../dracon-ai-contracts" }

[lib]
name = "dracon_ai_runtime_contracts"
path = "src/lib.rs"
