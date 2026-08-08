# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Embedded local model instructions

For local SolaiAgent/Ollama models whose Modelfile or template already
contains the stable SolaiAgent agent instructions, set:

```toml
model_provider = "ollama"
model = "SolaiAgent-30B-tools"
model_has_embedded_instructions = true
```

This advanced option reduces duplicate prompt tokens by replacing the repeated
base instructions with a short runtime pointer. It does not remove dynamic
context, tool schemas, project instructions, or runtime safety state. Only
enable it for local models that actually embed equivalent stable instructions.

## Lifecycle hooks

Admins can set top-level `allow_managed_hooks_only = true` in
`requirements.toml` to ignore user, project, and session hook configs while
still allowing managed hooks from requirements and managed config layers. This
setting is only supported in `requirements.toml`; putting it in `config.toml`
does not enable managed-hooks-only mode.
