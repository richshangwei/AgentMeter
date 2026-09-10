# Lessons

## 2026-09-10 Missing verification
- Signal: rendered compact UI height exceeded 640px although DOM unit tests passed.
- Prevention: measure real browser geometry at target viewport, with success and stale/error data before declaring layout complete.
- Tripwire: run node .scratch/verify-compact-ui.cjs.

## 2026-09-10 Test harness assumption
- Signal: added class queries encountered undefined className in lightweight mock elements.
- Prevention: model required DOM defaults in the test harness and run the targeted test immediately.

## 2026-09-10 Inherited build validation
- Signal: full verifier caught collapsible_if in the previously added path-normalization fix.
- Prevention: run warnings-denied Clippy before describing inherited changes as verified.

## 2026-09-10 New-PC guidance
- Signal: generic needs-attention state did not tell users what to install or how to recover.
- Prevention: provider-specific missing-CLI/auth messages must name the official tool, login action, and refresh action.

## 2026-09-10 Tablet HTTP regression
- Signal: immediate writes with shutdown hid Windows accepted-socket nonblocking inheritance; physical USB requests failed before parsing.
- Prevention: test delayed, fragmented requests without write shutdown and the actual forwarded Host/Origin. Never infer hardware or browser incompatibility from a generic parse error. Wait for packaging completion before handing off a build.
