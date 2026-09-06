# Model Source status as orthogonal axes

AgentMeter models Availability, Collection State, Freshness, optional Failure Code, and Collector Maturity independently instead of forcing them into one status enum. A Source can therefore be both rate-limited and stale, or available but backed off, without discarding information or inventing an ever-growing set of combined states.
