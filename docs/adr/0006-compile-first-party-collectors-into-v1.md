# Compile first-party Collectors into v1

AgentMeter v1 compiles its four first-party Collectors into the application and invokes official CLIs only through controlled subprocess boundaries. A Rust interface remains the testing seam, but v1 has no dynamic Collector plugin SDK or arbitrary script loading; this trades third-party extensibility for a bounded permission model, reproducible packaging, and auditable data handling.
