# SOLAI Agent

SOLAI Agent is the product name for the Rust workspace in this repository.
The runnable CLI is `solai`, built from `codex-cli`.

## What lives where

- `cli/` and `tui/`: the terminal application and its interactive UI
- `core/`: session orchestration, turn planning, sandboxing, tool selection,
  and model/runtime behavior
- `model-provider/`, `models-manager/`, `model-provider-info/`: provider
  catalog resolution, model metadata, and provider-specific runtime plumbing
- `app-server/` and `app-server-protocol/`: JSON-RPC app integration surface
- `exec/`, `exec-server/`, `execpolicy/`, `sandboxing/`, `network-proxy/`:
  command execution and sandbox enforcement
- `tools/`, `prompts/`, `collaboration-mode-templates/`: tool definitions and
  prompt templates used by the agent
- `sdk/`: Python and TypeScript SDKs

## Runtime shape

The CLI starts by loading layered configuration, resolving the active model
provider, refreshing model metadata, and then building a turn context. Each
turn decides which tools are visible from the resolved `ModelInfo`, the active
feature flags, and the selected execution environment.

The important control points are:

- model metadata comes from the provider catalog when available
- tool visibility comes from `tool_mode`, feature flags, and provider
  capabilities
- command execution is isolated through the sandbox and exec server layers
- app-server exposes the same core runtime over JSON-RPC for GUI clients

## Documentation map

- [Configuration guide](CONFIGURATION.md)
- [App server protocol](app-server/README.md)
- [Core implementation notes](core/README.md)
- [Tooling notes](tools/README.md)
- [Protocol types](protocol/README.md)

## Common workflows

Build the CLI:

```bash
cargo build -p codex-cli --bin solai
```

Run the app server:

```bash
cargo run -p codex-cli --bin solai -- app-server --help
```

List the active model provider and model catalog:

```bash
cargo run -p codex-cli --bin solai -- status
```

For provider configuration examples, use `CONFIGURATION.md`. For turn and
tool behavior details, inspect `core/` and `model-provider/`.
