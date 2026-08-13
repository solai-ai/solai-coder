# SOLAI Provider

Local compute provider daemon for the SOLAI Network.

The SOLAI protocol is being developed in this repository. Provider mode is kept
separate from the main `solai` agent so the compute layer can evolve without
breaking local chat/coder workflows.

## Commands

```bash
npm install
npm run provider -- enable
npm run provider -- status
npm run provider -- price SOLAI-20B 4
npm run provider -- schedule --from 22:00 --to 07:00
npm run provider -- daemon
npm run provider -- disable
```

When wired through the Rust CLI, the equivalent commands are:

```bash
solai provider enable
solai provider status
solai provider price SOLAI-20B 4
solai provider schedule --from 22:00 --to 07:00
solai provider disable
```

## Runtime

The daemon stores state under `~/.solai` by default:

- `provider.json` - provider config, pricing, schedule and identity
- `provider.pid` - daemon PID when started by `enable`
- `provider.log` - operational logs

Set `SOLAI_HOME` to use another directory.

Default HTTP port: `9898`.

Endpoints:

- `GET /health`
- `GET /metrics`
- `GET /heartbeat`
- `GET /jobs`
- `POST /jobs`
- `POST /v1/chat/completions`

The daemon only proxies inference requests to Ollama. It does not expose shell
execution or filesystem mutation endpoints.
