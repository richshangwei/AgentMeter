# Local Windows coordination patch

Base: cached crates.io tauri-plugin-single-instance 2.4.3. Original licenses retained.

Windows changes only: reject failed mutex creation; wait up to two seconds for an existing owner's IPC window; reject unreachable owners instead of continuing startup; bound message delivery to two seconds; make exit-only invocation with no owner a no-op; and make a readiness probe with no owner exit with code 3 rather than becoming the owner. Other platforms are unchanged.

This preserves Windows desktop isolation. It does not relay commands across desktops or elevate privileges. An isolated launch returns an error instead of starting another collector or writer. Review this patch when updating the upstream dependency.
