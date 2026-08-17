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
npm run provider -- register --endpoint https://provider.example.com:9898 --name gpu-node-1
npm run provider -- probe https://provider.example.com:9898
npm run provider -- refresh
npm run provider -- list
npm run provider -- quote --choice 1 --hours 1
npm run provider -- rent --choice 1 --hours 1
npm run provider -- leases
npm run provider -- lease <LEASE_ID>
npm run provider -- run --model SOLAI-20B --lease <LEASE_ID> --prompt "Explain SOLAI in one paragraph"
npm run provider -- release <LEASE_ID>
npm run provider -- daemon
npm run provider -- disable
```

When wired through the Rust CLI, the equivalent commands are:

```bash
solai provider enable
solai provider status
solai provider price SOLAI-20B 4
solai provider schedule --from 22:00 --to 07:00
solai provider register --endpoint https://provider.example.com:9898 --name gpu-node-1
solai provider probe https://provider.example.com:9898
solai provider refresh
solai provider list
solai provider quote --choice 1 --hours 1
solai provider rent --choice 1 --hours 1
solai provider leases
solai provider lease <LEASE_ID>
solai provider run --model SOLAI-20B --lease <LEASE_ID> --prompt "Explain SOLAI in one paragraph"
solai provider release <LEASE_ID>
solai provider disable
```

Marketplace operations are also exposed as a grouped SOLAI CLI surface:

```bash
solai marketplace register --endpoint https://provider.example.com:9898 --name gpu-node-1
solai marketplace probe https://provider.example.com:9898
solai marketplace refresh
solai marketplace list
solai marketplace quote --choice 1 --hours 1
solai marketplace rent --choice 1 --hours 1
solai marketplace leases
solai marketplace lease <LEASE_ID>
solai marketplace run --model SOLAI-20B --lease <LEASE_ID> --prompt "Explain SOLAI in one paragraph"
solai marketplace release <LEASE_ID>
```

Inside the interactive SOLAI Agent session, the same marketplace flow is
available through slash commands:

```text
/marketplace probe https://provider.example.com:9898
/marketplace refresh
/providers
/quote --choice 1 --hours 1
/rent --choice 1 --hours 1
/leases
/lease <LEASE_ID>
/marketplace run --model SOLAI-20B --lease <LEASE_ID> --prompt "Explain SOLAI in one paragraph"
/release <LEASE_ID>
```

## Runtime

The daemon stores state under `~/.solai` by default:

- `provider.json` - provider config, pricing, schedule and identity
- `provider.pid` - daemon PID when started by `enable`
- `provider.log` - operational logs
- `provider-registry.json` - discovered marketplace providers, endpoints, signed heartbeat snapshots and reputation counters
- `provider-leases.json` - exclusive leases active on a provider machine
- `marketplace-leases.json` - local client leases and lease tokens used to submit work
- `marketplace-client.json` - local marketplace renter identity

Set `SOLAI_HOME` to use another directory.

Default HTTP port: `9898`.
Default HTTP bind host: `127.0.0.1`. Set `SOLAI_PROVIDER_HOST=0.0.0.0`
when running a network-reachable provider behind your firewall or reverse proxy.

Endpoints:

- `GET /health`
- `GET /metrics`
- `GET /heartbeat`
- `GET /marketplace/provider`
- `GET /marketplace/providers`
- `GET /jobs`
- `POST /jobs`
- `POST /v1/chat/completions`

The daemon only proxies inference requests to Ollama. It does not expose shell
execution or filesystem mutation endpoints.

## Marketplace flow

Provider discovery uses signed heartbeats. `probe` fetches `/heartbeat`,
verifies the provider signature against the advertised public key and stores the
provider in `provider-registry.json`. `refresh` rechecks all registered endpoints
and marks unreachable providers offline. `quote` selects the lowest-priced
online provider matching the requested model and availability window. `rent`
creates an exclusive hourly lease and returns a lease token. While the lease is
active, the provider rejects jobs from other clients with `423 Locked`. `run`
can submit work through an active lease by sending the lease id and token.
