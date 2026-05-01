# Project State
##Current Focus
The commit refactors the configuration and secrets file locations in `SimpleAiService`, moving them from top‑level `.dracon` directories to the nested `utilities/sync` subdirectory, aligning with the project's XDG‑compliant state directory layout.

## Completed
- [x] Updated `config_path` to return `dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".dracon/utilities/sync/ai.toml")`.
- [x] Updated `secrets_path` to return `dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".dracon/utilities/sync/ai/secrets")`.
