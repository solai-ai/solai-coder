# SOLAI Agent Runtime for Python SDK

Platform-specific runtime package consumed by the published `openai-codex`.

This package is staged during release so the SDK can pin an exact SOLAI Agent
version without checking platform binaries into the repo.

`openai-codex-cli-bin` is intentionally wheel-only. Do not build or publish an
sdist for this package.
