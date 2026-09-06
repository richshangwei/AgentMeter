# AgentMeter

This repository currently contains the P0 feasibility collector for Codex. It proves the local app-server protocol path before the full Windows desktop application is built.

## Build and test

```powershell
cargo build --offline --locked
cargo test --offline --locked
```

The dependencies are pinned in `Cargo.lock`. Remove `--offline` only when a dependency refresh is intentional.

## Run the Codex experiment

Use the current user's Codex installation:

```powershell
cargo run --offline --locked --bin agentmeter-p0 -- codex collect
```

Use a sanitized transcript to reproduce normalization without a live account:

```powershell
cargo run --offline --locked --bin agentmeter-p0 -- codex collect --fixture tests/fixtures/codex/success.jsonl
```

The command writes one versioned JSON collection report to standard output. Exit code `0` means a trustworthy Observation was produced. Exit code `1` means collection failed and the JSON contains a distinct `failure_code`; `observation` remains `null`. Exit code `2` means the command arguments are invalid.

The collector masks account email addresses in successful output and never copies Codex credentials. `account/usage/read` is probed only to record whether the installed app-server supports it; an unsupported method is reported rather than retried.
