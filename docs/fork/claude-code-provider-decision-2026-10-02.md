# Claude Code provider decision — 2026-10-02

**Decision: NO_ADOPT for the current Meetily roadmap.** FEAT-117 research ends
with this negative decision. The conditional FEAT-118 prototype is not needed.
This is a product/engineering choice, not a blanket claim that embedding Claude
Code is prohibited.

## Official terms checked today

Anthropic distinguishes running its unmodified CLI with users authenticating
through its own flow from offering a third-party application's Claude.ai login
or intermediating users' subscription credentials. Its product-hosting guidance
requires commercial terms and preserves the binary's authentication options;
its developer guidance directs Claude integrations to API-key or supported
cloud authentication. Meetily will not collect subscription tokens or promise
free subscription-backed inference. See [legal and compliance](https://code.claude.com/docs/en/legal-and-compliance)
and the [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview).

The published [Commercial Terms](https://www.anthropic.com/legal/commercial-terms)
show an effective date of June 17, 2025 and permit powering customer products
subject to those terms. The fetched [Consumer Terms](https://www.anthropic.com/legal/consumer-terms)
show October 8, 2025; applicable consumer terms depend on the user and region.
These observations do not establish eligibility for any particular account.

## CLI interface and its limits

The current [CLI reference](https://code.claude.com/docs/en/cli-reference)
documents non-interactive `--print`, piped input, JSON output and
`--no-session-persistence`. `--tools ""` disables built-in tools, but MCP must
be separately excluded. `--allowedTools` is an approval list, not the tool
availability boundary. Restricted mode requires version 2.1.248 or later and
limits tools/settings; these flags do not prove OS isolation or removal of
all diagnostic data. Permissions and isolation are distinct controls, as the
[sandbox documentation](https://code.claude.com/docs/en/sandbox-environments)
explains.

Context7 resolved the official documentation and cross-checked CLI semantics;
the live official pages supplied the final evidence. `command -v claude`
returned exit 1: no CLI was found on PATH. No installed version is established,
no CLI was installed or invoked, and documented behavior is not a runtime PASS.

## Why not adopt

Current `main@53f4415058fb01e38b8892a47ac036bfffa394ba` already has the Claude
provider in `frontend/src-tauri/src/summary/llm_client.rs`: it builds an
`x-api-key` request to Anthropic's Messages API. Source inspection establishes
that path; it does not prove a paid request or account eligibility.

Meetily's proposed assistant needs selected text, a configured provider and an
answer. Disabling coding tools removes the CLI's main added capability while
adding installation/version management, subprocess supervision, settings/MCP
isolation and a second authentication boundary. No measured quality or latency
benefit over the existing API path exists. This cost/benefit assessment is the
reason for NO_ADOPT; a CLI prototype is not necessary to reject an optional
provider at this research gate.

## Minimal text interface if the decision is later reopened

Pass only bounded UTF-8 `{question, context, model}` via stdin; return structured
`{answer, provider, model, error}` via stdout. No project path, shell access,
plugins, MCP, credentials in payload/argv, conversation resume or user profile
corpus. Credentials must use the supported authentication flow. App logs may
record status, duration and byte counts, never text, stdout/stderr dumps or
secrets. A future prototype must prove OS isolation, deadline/process-group
cancellation, temporary-data cleanup and output redaction on a pinned binary.
Those are proposed constraints, not executed checks.

## Evidence boundary

Official terms and CLI documentation were read; the current API path and local
CLI absence were checked. No credentials, model requests, installation,
subscription usage, native packaging or cancellation/log-retention runtime
checks were performed. No product code changed. The negative research branch
does not claim a working CLI integration or make any release ready.
