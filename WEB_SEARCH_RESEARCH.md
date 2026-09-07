# Credential-Free Web Search Research

Status: implementation proposal only  
Last verified: 2026-09-07  
Scope: web search for local and third-party models without OpenAI credentials

## Goal

Give local models such as Qwen and GPT-OSS useful web search while keeping SOLAI
model-agnostic, economical on older hardware, and independent of OpenAI
credentials. Search must remain optional and must not add tool instructions to
the model context when disabled.

## Current SOLAI behavior

- SOLAI contains hosted `web_search` support and a standalone `web.run`
  extension.
- The standalone extension currently sends its search request to an OpenAI
  search endpoint. It is therefore not a credential-free local backend.
- The llama.cpp adapter filters hosted tool types. Consequently, a local model
  does not receive an effective web search tool through that path.
- Local models can use ordinary function tools after the generic tool-call
  adaptation, so a provider-neutral local search executor is feasible.

## What the hosted Codex path does

OpenAI exposes web search as a hosted Responses API tool. The service executes
the search and returns structured search events, results, and sources to the
client/model. This is not equivalent to scraping a public search-engine HTML
page from the Codex CLI.

Reference:

- <https://developers.openai.com/api/reference/cli/resources/responses/methods/create>

## Google and Google AI Mode findings

Google Search and Google AI Mode were tested from the SOLAI server using both
direct HTTP requests and the installed Google Chrome in headless mode.

Test query:

```text
Qwen3 tool calling llama.cpp
```

Endpoints tested:

```text
https://www.google.com/search?q=Qwen3+tool+calling+llama.cpp&hl=en&gl=us
https://www.google.com/search?q=Qwen3+tool+calling+llama.cpp&hl=en&gl=us&udm=50
```

Observed result:

- Direct requests returned an HTTP 200 JavaScript shell without usable search
  results or an AI summary.
- Headless Chrome rendered approximately 5.2 KB for each endpoint, but both
  pages contained a CAPTCHA rather than results.
- Changing user agents and common query parameters did not make the output a
  stable machine-readable result.
- Google AI Mode is a user-facing browser experience, not a documented
  anonymous search API.
- Google's documented Custom Search JSON API requires an API key and a
  configured search engine. It is not a no-credential replacement.
- Gemini grounding with Google Search is also an authenticated API feature.

References:

- <https://support.google.com/websearch/answer/16011537>
- <https://developers.google.com/custom-search/v1/overview>
- <https://ai.google.dev/gemini-api/docs/google-search>

Conclusion: do not ship Google Search or Google AI Mode HTML scraping as the
default SOLAI backend. It is already blocked in the target environment and
would make the feature fragile.

## Other public endpoints tested

| Provider | Result from the target server | Suitability |
| --- | --- | --- |
| DuckDuckGo HTML | Bot challenge/CAPTCHA | Not reliable as an automatic default |
| Brave Search HTML | HTTP 429/CAPTCHA | Not reliable as an automatic default |
| Mojeek HTML | CAPTCHA | Not reliable as an automatic default |
| Yahoo Search | Failed response/no usable body | Not reliable |
| Public SearXNG instances | Mostly HTTP 429 or security challenges | Useful only as configurable, best-effort instances |
| Marginalia | Responded | Too specialized to be the only general provider |
| Bing RSS | Returned useful results when locale was forced | Possible best-effort fallback for personal use |

For Bing RSS, forcing an English/US locale materially improved relevance:

```text
setlang=en-US&cc=US
```

Its published response includes usage language aimed at personal,
non-commercial RSS aggregation. Distribution and product use must be reviewed
before making it a permanent default.

## Recommended architecture

Implement a provider-neutral local tool rather than teaching each model about
a particular search engine.

### Provider order

1. A user-configured SearXNG endpoint as the preferred credential-free
   provider. Self-hosting gives the most predictable behavior and control.
2. An explicitly enabled public, best-effort provider for home use. Bing RSS is
   currently the most functional tested candidate, subject to usage review.
3. Existing hosted web search when the selected inference provider supports it
   and the user has configured the required authentication.

Do not silently fall back from a local provider to OpenAI or another
authenticated service.

### Model-facing interface

Expose one stable tool schema to every compatible model:

```text
web.search(query, recency?, domains?, max_results?)
web.open(reference_or_url)
web.find(reference_or_url, pattern)
```

The executor, not the model, selects the configured backend. This keeps Qwen,
GPT-OSS, OpenAI-compatible servers, and future providers on the same interface.

Search output should contain only compact structured fields:

```json
{
  "results": [
    {
      "ref": "search-1",
      "title": "...",
      "url": "https://...",
      "snippet": "..."
    }
  ]
}
```

The local model summarizes these results. A search-engine-generated AI summary
is not required and should not be trusted as the only source of evidence.

## Context and token budget requirements

- Do not include the web tool or its instructions when web search is disabled.
- Under `lite-agentctx`, use the shortest schema and description that still
  preserves correct calls.
- Add search results to context only after a search call.
- Cap the number of results and the length of every title and snippet.
- `web.open` should extract readable text and enforce a hard byte/token cap.
- Prefer opening only sources selected by the model, not every search result.
- Store result references and fetched pages outside model-visible history; send
  only the requested excerpt back to the model.
- Cache normalized queries and fetched documents locally with a configurable
  TTL.
- Deduplicate canonical URLs and repeated snippets before they enter context.
- When compaction occurs, retain conclusions and cited URLs, not full fetched
  pages.

Suggested conservative defaults for local hardware:

| Limit | Default |
| --- | ---: |
| Search results | 5 |
| Title | 160 characters |
| Snippet | 500 characters |
| Downloaded response | 2 MiB |
| Extracted page text | 12,000 characters |
| Search timeout | 15 seconds |
| Page fetch timeout | 20 seconds |
| Cache TTL | 30 minutes |

All limits must be configurable but must retain hard upper bounds.

## Safety requirements

- Permit only `http` and `https` input URLs.
- Resolve hostnames before connecting and reject loopback, link-local, private,
  multicast, and otherwise non-public addresses unless the user explicitly
  configures a trusted self-hosted search endpoint.
- Revalidate every redirect target to prevent SSRF through redirects or DNS
  changes.
- Limit redirects, response bytes, decompressed bytes, and total elapsed time.
- Do not send cookies, local credentials, browser profiles, or repository data
  to search providers.
- Strip scripts, styles, navigation, and hidden content during page extraction.
- Treat fetched text as untrusted content and clearly delimit it from system and
  developer instructions.
- Preserve source URLs so final answers can cite the pages actually used.

## Suggested implementation stages

### Stage 1: provider-neutral foundation

- Define a local search provider trait outside `codex-core` when practical.
- Implement configuration, compact tool schemas, bounded results, and an
  in-memory per-thread reference store.
- Ensure feature detection exposes the tool to models that support ordinary
  function calling, including llama.cpp adapters.
- Add integration tests proving that disabled search contributes zero
  model-visible tool instructions.

### Stage 2: SearXNG

- Implement its JSON response format against a user-configured endpoint.
- Support a trusted private endpoint without weakening public URL checks for
  arbitrary page fetching.
- Test failure, timeout, empty-result, malformed-result, and rate-limit paths.

### Stage 3: page reading

- Add `open` and `find` with redirects, SSRF protection, content-type checks,
  extraction, truncation, and caching.
- Return compact metadata when a page cannot be read instead of injecting a
  large error into context.

### Stage 4: optional public fallback

- Add only after usage terms and redistribution implications are reviewed.
- Mark it best-effort and make the selected provider visible in diagnostics.
- Never hide a provider switch from the user.

## Acceptance criteria

- Works with at least one local Qwen model and one other OpenAI-compatible local
  model without OpenAI credentials.
- Search is absent from the initial context when disabled.
- Enabling search adds only the compact tool definition, not provider manuals.
- A five-result search stays within its configured character/token budget.
- Results expose stable references and source URLs.
- `open` cannot access localhost, private network services, or cloud metadata by
  default.
- Timeouts, cancellation, rate limits, malformed responses, and unavailable
  providers do not crash the agent.
- Tool-call and tool-output wire types remain aligned for all model adapters.
- Existing hosted OpenAI search continues to work unchanged.
- Prompt cache reuse is not invalidated by changing dynamic search-result data
  before any search is performed.

## Explicit non-goals

- Automating Google login, CAPTCHA solving, or browser profiles.
- Depending on undocumented Google AI Mode HTML or internal endpoints.
- Sending every fetched page into model context.
- Making a single public third-party instance a guaranteed service dependency.
- Coupling web search behavior to one model family or one chat template.

