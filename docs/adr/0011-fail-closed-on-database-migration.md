# Fail closed on database migration

AgentMeter creates a versioned, timestamped backup before every SQLite migration and retains at most the three newest migration backups. If either backup or migration fails, collection and all database writes remain stopped until the user restores or repairs the data; continuing against a partial schema is never treated as degraded operation.
