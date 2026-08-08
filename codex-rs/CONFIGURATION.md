# Configuration Guide

This document describes how SOLAI Agent is configured and how the main
pieces fit together.

## Configuration layers

SOLAI Agent resolves configuration from multiple layers. In practice, the
important files are:

- user config: `~/.codex/config.toml`
- project config: `.codex/config.toml`
- managed config, when present
- command-line overrides, which take precedence for one run

The effective config is built by layering these sources and then resolving the
selected model provider, sandbox policy, approvals policy, and feature flags.

## Model selection

The active model comes from `model`, while the active provider comes from
`model_provider`.

- `model` selects the model slug to use for the current session
- `model_provider` selects the configured provider entry from
  `[model_providers.<id>]`
- `oss_provider` selects the default local OSS provider for local-model flows;
  the built-in values are `ollama`, `lmstudio`, and `llama`

The `llama` OSS provider name is an alias for the Ollama-style local provider
path. If you are running a llama.cpp server that exposes a `/v1` API, the most
explicit setup is to define a custom provider and point `model_provider` at it.

## Ollama example

This is the simplest local setup when Ollama is running on the default port.

```toml
model_provider = "ollama"
oss_provider = "ollama"
model = "qwen3-coder:30b"

[model_providers.ollama]
name = "Ollama"
base_url = "http://127.0.0.1:11434/v1"
```

Notes:

- the default Ollama port is `11434`
- the provider should expose `/api/tags` and `/v1/*` endpoints
- the model catalog can advertise `capabilities = ["tools"]`; when it does,
  SOLAI Agent treats that model as tool-capable

## LM Studio example

LM Studio uses the built-in `lmstudio` OSS provider id and defaults to port
`1234`.

```toml
model_provider = "lmstudio"
oss_provider = "lmstudio"
model = "qwen3:14b"

[model_providers.lmstudio]
name = "LM Studio"
base_url = "http://127.0.0.1:1234/v1"
```

## llama.cpp example

If your llama.cpp server exposes an OpenAI-compatible `/v1` API, define a
custom provider id and point it at that server.

```toml
model_provider = "llama-cpp"
model = "qwen3-coder:30b"

[model_providers.llama-cpp]
name = "llama.cpp"
base_url = "http://127.0.0.1:8080/v1"
```

If you want the local-provider shortcut to resolve to the Ollama-style OSS
path, you can also set:

```toml
oss_provider = "llama"
```

That alias resolves to the built-in local OSS provider path, which keeps
existing OSS-provider workflows working on systems that call the local backend
"llama".

## Provider selection rules

Provider ids are resolved from the built-in catalog first, then from
`[model_providers.<id>]` entries.

Built-in provider ids include:

- `openai`
- `amazon-bedrock`
- `ollama`
- `lmstudio`

The local OSS provider alias `llama` is accepted for `oss_provider`, but it
resolves to the Ollama-style local provider path internally.

## What the provider must return

For the runtime to behave well, provider metadata should include:

- the real model slug
- a stable display name
- `tool_mode` when the provider knows the model is tool-capable
- `used_fallback_model_metadata = false` for authoritative catalog entries

If `tool_mode` is omitted and the provider does not advertise a tool-capable
model, SOLAI Agent may fall back to direct mode behavior.

## Practical defaults

For day-to-day local use:

```toml
model_provider = "ollama"
oss_provider = "ollama"
model = "qwen3-coder:30b"
approval_policy = "on-request"
sandbox_mode = "workspace-write"
```

For a custom local `/v1` server:

```toml
model_provider = "llama-cpp"
model = "qwen3-coder:30b"
approval_policy = "on-request"
sandbox_mode = "workspace-write"
```

## Troubleshooting

- If the CLI says a model is missing metadata, the provider catalog did not
  return an authoritative `ModelInfo` for that slug.
- If `exec` is missing, check whether the resolved `ModelInfo` has
  `tool_mode` set or whether the provider catalog advertises `tools`.
- If the provider does not support `/models`, SOLAI Agent falls back to its
  built-in catalog and the selected model may inherit generic metadata.
