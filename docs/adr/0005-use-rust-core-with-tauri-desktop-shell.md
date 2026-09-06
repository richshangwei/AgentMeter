# Use a Rust core with a Tauri desktop shell

AgentMeter uses a Rust core with a Tauri 2 desktop shell and a Web frontend, with Tokio, Axum, Serde, and SQLite as the default implementation stack unless a P0 spike disproves a choice. This keeps collection, persistence, scheduling, USB, and the loopback API in one native core while reusing Web UI components across desktop and tablet instead of committing to separate native interfaces.
