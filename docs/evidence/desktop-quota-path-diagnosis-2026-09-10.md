# Desktop 0/4 diagnosis — 2026-09-10

Status: cause reproduced; production fix not applied. The earlier native collector
PASS is not a desktop UI acceptance result. Do not treat the previous installer as
verified working in the actual window.

## Actual WebView reproduction

Launched the diagnostic release with `--quota-diagnostics` as the normal Windows
user. The same startup collector and real dashboard rendering ran; no mock data.
PID at capture: 45244. Opt-in diagnostic output is capped at 64 KiB (plus its final
record), under the application's local-data logs directory. It excludes account
identity, credentials, raw provider output, and full filesystem paths.

Selected diagnostic records:

```text
runtime: node_exists=true, entry_exists=true, runtime_verbatim=true, workdir_verbatim=false
child_exit: code=1
output: bytes=0
collect_failed: code=collector_failed
ui_rendered: ready=0, windows=[0,0,0,0]
```

## Minimal reproduction

Command: `node .scratch/quota-desktop-diagnostic/path-probe.cjs`

This uses an invalid provider argument, which should exit 2 after loading the
entry point, before any account or network access. Failure to load instead exits 1.
The probe returns nonzero if an entry cannot load. Both restricted and normal-user
runs produced the same results:

| Input | Exit | Result |
| --- | --- | --- |
| Ordinary absolute entry and working directory | 2 | Entry loaded |
| Verbatim (`\\?\`) absolute entry | 1 | EISDIR, before entry executes |
| Relative entry with verbatim working directory | 1 | EISDIR, before entry executes |

The bundled Node reports `EISDIR: illegal operation on a directory, lstat 'D:'`.
Tauri's resource directory has the verbatim prefix. The standalone native probe
obtains its resource path differently, explaining why its successful results did
not detect the UI startup failure. The desktop UI also masks the shared failure
with generic login advice, which is misleading in this case.

## Next implementation and acceptance

Normalize supported Windows verbatim paths at the controlled Node subprocess
boundary, including its working directory and filesystem arguments; do not just
change the script argument to a relative path. Preserve UNC semantics and do not
silently truncate long paths. Add a real subprocess regression at this seam, then
verify actual WebView-rendered quota counts, not only the native collector report.
No fix or replacement installer has been delivered in this diagnostic turn.

Temporary instrumentation is opt-in and tagged `DEBUG-quota-desktop`; remove it
after the eventual original-scenario acceptance test. The minimized probe remains
in the explicitly diagnostic scratch directory. Desktop Rust tests (18) and UI
logic tests (2) pass with the instrumentation; these do not certify a fixed UI.
