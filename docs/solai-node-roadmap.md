# SOLAI Node Roadmap

SOLAI Node is planned as the private inference runtime for the SOLAI platform.
It is the bridge between applications, compute providers, model providers, pricing,
telemetry, and partner workloads.

The implementation details and provider integrations are private platform work.
This public roadmap exists so the community can follow the direction without exposing
security-sensitive code, partner integrations, or infrastructure internals.

## Product role

SOLAI Coder is the public local-first coding experience.

SOLAI Node is the inference layer that powers provider discovery, workload routing,
usage tracking, and external partner access.

The Coder can use a local embedded Node for a simple user experience. Partners and
larger deployments can use SOLAI Node as a standalone service when they only need
inference access.

## Public milestones

### Phase 1: Foundation

- Define the boundary between SOLAI Coder and SOLAI Node
- Document embedded, standalone local, and standalone remote deployment modes
- Keep the public Coder workflow stable while Node work moves behind a private runtime

### Phase 2: Contracts

- Define versioned job contracts
- Define provider capability contracts
- Define health, metrics, heartbeat, and usage event contracts
- Keep the contracts stable enough for Coder and partner integrations

### Phase 3: Runtime

- Add the standalone Node runtime
- Add local configuration and secure credential loading
- Add health checks, metrics, logs, and job status endpoints
- Support local and remote operation

### Phase 4: Provider network

- Add provider registration and discovery
- Add model inventory reporting
- Add pricing per model and workload class
- Add availability scheduling for provider machines
- Add fallback behavior for unavailable providers

### Phase 5: Partner workloads

- Support image, video, text, audio, and other inference workloads through provider adapters
- Track tenant usage and metering
- Add partner-facing API documentation
- Add operational controls for quotas, rate limits, and billing records

## Public progress format

Progress should be shared publicly as high-level updates:

- architecture notes
- protocol milestones
- endpoint categories
- integration status
- provider capability categories
- community-facing changelog entries

The public repository should not expose:

- provider secrets
- billing implementation details
- private partner logic
- infrastructure topology
- private model routing logic
- signing keys or credential flows

## Current status

The public Coder repository documents the Node direction and deployment model.
The Node runtime itself should remain private while its contracts and community-facing
milestones are communicated here.

