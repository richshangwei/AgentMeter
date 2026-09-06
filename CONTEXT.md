# AgentMeter

AgentMeter is a local Windows monitoring product that presents trustworthy usage and quota information from multiple AI coding providers on the desktop and a paired tablet.

## Language

**Provider**:
One of the external AI coding services monitored by AgentMeter: Claude Code, Codex, GitHub Copilot, or Google Antigravity.
_Avoid_: Agent, service, vendor

**Provider Account**:
The externally identifiable account whose usage or quota belongs to a Provider. One Provider Account may be observed through multiple Sources.
_Avoid_: Login, profile, user

**Account Link**:
A user-confirmed, reversible statement that Sources without a stable Provider identity belong to the same Provider Account. AgentMeter never infers an Account Link from paths, display names, or the current number of accounts.
_Avoid_: Account merge, automatic match

**Source**:
A Provider Account's observable presence in a specific local environment, such as native Windows or one WSL distribution.
_Avoid_: Provider, Collector, installation

**Quota Window**:
A Provider-defined limit period with its own unit, usage, remaining allowance, and optional reset time.
_Avoid_: Balance, total quota, limit

**Observation**:
A time-stamped Provider measurement with explicit scope, quality, and freshness. An Observation can be unknown and is never replaced with a fabricated zero or full value.
_Avoid_: Reading, result, current value

**Data Quality**:
The authority behind an Observation: `official`, `local_observed`, `estimated`, or `manual`. It says where the value came from, not whether its Collector is stable.
_Avoid_: Reliability, maturity, confidence

**Collector Maturity**:
The support level of a Provider integration: `stable`, `experimental`, or `unsupported`. It is independent of Data Quality, so an official source can still be experimental.
_Avoid_: Data quality, provider status

**Availability**:
Whether a Source can be collected in its present environment, independently of current collection activity and data freshness.
_Avoid_: Status, health, freshness

**Collection State**:
The current collection activity of a Source: idle, collecting, ready, backing off, or error. A separate Failure Code explains a failure without replacing the state.
_Avoid_: Availability, freshness, provider status

**Freshness**:
Whether an Observation is recent enough for its Source's update behavior. Event-driven Sources are judged against observed activity rather than a polling interval they do not have.
_Avoid_: Health, reliability, maturity

**Usage Series**:
A history formed only from comparable usage Observations with the same Provider Account, scope, metric, and unit.
_Avoid_: Daily total, combined usage

**Quota Trend**:
The time series of a single Quota Window. Snapshots describe its state and are never summed as usage.
_Avoid_: Usage total, quota usage series

**Daily Summary**:
A day-bucketed Usage Series result that retains the timezone used to define that day. A Daily Summary is not silently reinterpreted when the display timezone changes.
_Avoid_: Daily total, global usage

**Dashboard Snapshot**:
The complete, revisioned view of every Provider at one generated time. A new stream identity marks an App restart, so revisions are compared only within the same stream.
_Avoid_: Provider snapshot, SSE patch, event

**Over Limit**:
A Quota Window whose reported usage exceeds its limit. The main remaining value is displayed as zero while the original overage remains visible and stored.
_Avoid_: Negative remaining, extra allowance

**Monitor-only Tablet Client**:
The paired tablet view that can inspect dashboard data and request a throttled refresh but cannot configure Sources, authenticate accounts, clear data, run commands, control Providers, or exit AgentMeter.
_Avoid_: Read-only tablet, remote control

**Device Pair**:
The durable authorization granted to one tablet browser profile. It survives normal restarts and USB reconnections until the user revokes it, switches devices, or clears pairing data.
_Avoid_: Session, ADB connection, USB mapping

**Tablet Session**:
A short-lived, rotating authorization derived from a Device Pair for dashboard, history, event-stream, and refresh requests.
_Avoid_: Device Pair, pairing code, login

**History**:
Stored usage, quota, health, and alert records. Clearing History preserves settings, Provider Accounts, Account Links, and Device Pairs.
_Avoid_: All data, settings, cache

**Forget Account**:
The explicit removal of one Provider Account, its Account Links, and all History belonging to it.
_Avoid_: Logout, clear history, disable source

**Reset AgentMeter**:
The separately confirmed operation that returns AgentMeter to its initial local state by removing History, account identity, settings, and pairing data.
_Avoid_: Clear history, uninstall, restart

**Setup Required**:
A Source state indicating that AgentMeter needs explicit user-approved configuration before it can collect data without damaging an existing workflow.
_Avoid_: Unsupported, error, needs login

**Paused**:
The user-selected state that stops collection, retries, and notifications while keeping existing Observations visible on desktop and tablet until monitoring is explicitly resumed.
_Avoid_: Offline, stopped, stale
