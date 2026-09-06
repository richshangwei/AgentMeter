# Stream complete Dashboard Snapshots

The tablet API returns and streams complete Dashboard Snapshots rather than incremental patches because the four-Provider payload is small and full state makes missed-event recovery deterministic. Each App run has a new `stream_id` and an increasing `revision`; refresh requests return quickly with `202 Accepted`, while collection results arrive in a later snapshot and heartbeats never advance the revision.
