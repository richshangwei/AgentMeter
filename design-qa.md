# AgentMeter redesign — design QA

## 2026-09-11 Fit-to-viewport rewrite (supersedes the scrollable real-data correction)

User feedback rejected scrollbars and cramped resize behaviour. The desktop dashboard now never scrolls: the card grid, cards and quota regions are fixed to the viewport; every quota window is a tile sized from the measured region (ring / stack / bar / line variants) with the full label, reset and usage in a tooltip. Small windows keep the count topology but show fewer cards per page when a card or its densest quota tiles would fall below 90×56 px. Verified with `scripts/verify-desktop-responsive.cjs` (18 viewports × 1/2/3/4 cards + edge states) in Chromium on Linux; Windows/Edge (Segoe UI metrics) and a packaged build still need a run.

## 2026-09-11 Real-data correction (supersedes prior desktop completion claims)

The earlier two-window fixtures missed real data cardinality. Eight-window Codex data reproduced a clipped primary ring at 2048 × 1190; a start-alignment-only probe then exposed the fractional percentage sizing gap. The corrected desktop uses start-aligned scrollable quota regions, explicit four-row gauge placement, gauge-relative percentage sizing, wrapping labels, and nonshrinking footer actions. Cards keep count-driven columns/rows at supported desktop widths and use a minimum readable height with grid scrolling when needed; below 600px they stack. No quota rows or refresh buttons are hidden for short screens.

Unchanged polling preserves quota DOM nodes; changed values preserve scroll offset. Usage-only values use a text row rather than a percentage ring. Regions are keyboard-focusable and have visible count/scroll guidance.

Verification: `scripts/verify-desktop-responsive.cjs` passed 11 sizes (2048×1190, 1873×1135, 1498×908, 952×1235, 762×988, 1080×640, 640×520, 1440×900, 1280×720, 375×844, 844×390), each with 1/2/3/4 cards. Includes eight quota rows, 24.7%, long resets, unknown/error/usage-only data, all-row reachability, footer containment, PageDown scrolling, and unchanged/changed refresh continuity. This supersedes old desktop assertions that required every row to fit without scrolling.

Visual captures inspected: `.scratch/responsive-real-data-1498.png`, `.scratch/responsive-real-data-952.png`. Existing palette and branding retained. The ui-ux-pro-max layout, scroll accessibility and text-scaling guidance informed the repair; its optional search executable is absent from the installed skill package.

Console-flash status is separate: not reproduced and not claimed fixed; see `scripts/test-refresh-console.ps1` and task notes.

Latest correction: period labels now occupy the second grid column, with progress and reset information on explicit rows. The percentage occupies the first column with the gauge. Removed the width-dependent 17% label margin that caused sibling overlap. The browser verifier now checks label/value rectangle intersections on every desktop card-count/viewport combination, in addition to existing clipping checks. Updated capture: `.scratch/desktop-percentage-hidpi-fixed.png`.

**Comparison target**

- Source visual truth: `C:\Users\richs\AppData\Local\Temp\codex-clipboard-f275fc64-32de-4b85-8dee-590c7ef96a62.png`
- Implementation screenshot: `D:\WorkSpace\AgentMeter\.scratch\desktop-reference-size.png`
- Viewport: 1452 × 1086 CSS px, device scale factor 1.
- Pixel dimensions: source 1452 × 1086; implementation 1452 × 1086. No density normalization was required.
- State: four providers in a full 2 × 2 composition; Codex, Claude Code, and Antigravity contain synchronized quota data; GitHub Copilot shows the setup/empty state. Dashboard management tiles are intentionally absent.
- Full-view comparison: `D:\WorkSpace\AgentMeter\.scratch\design-comparison-desktop.png`
- Focused card comparison: `D:\WorkSpace\AgentMeter\.scratch\design-comparison-card.png`
- Ultra-wide clipping source: `C:\Users\richs\AppData\Local\Temp\codex-clipboard-c38525b2-97dc-4229-ac49-d6061f923462.png` (2491 × 1312 physical px).
- Corrected ultra-wide implementation: `D:\WorkSpace\AgentMeter\.scratch\desktop-ultrawide.png` (2491 × 1312 CSS px), with 1993 × 1050 also checked as the 125%-scale CSS equivalent.
- High-DPI-equivalent capture: `D:\WorkSpace\AgentMeter\.scratch\desktop-hidpi-equivalent.png` (1993 × 1050 CSS px).
- Ultra-wide side-by-side comparison: `D:\WorkSpace\AgentMeter\.scratch\design-comparison-ultrawide.png`.
- Percentage-overlap source: `C:\Users\richs\AppData\Local\Temp\codex-clipboard-4814636e-b166-429a-a44b-f2cc76718a09.png` (2440 × 1288 physical px).
- Corrected percentage implementation: `D:\WorkSpace\AgentMeter\.scratch\desktop-percentage-fixed.png`; high-DPI-equivalent capture: `D:\WorkSpace\AgentMeter\.scratch\desktop-percentage-hidpi-fixed.png`.
- Percentage side-by-side comparison: `D:\WorkSpace\AgentMeter\.scratch\design-comparison-percentage.png`.

**Findings**

- No actionable P0, P1, or P2 mismatches remain.
- Fonts and typography: the Segoe UI Variable/system stack, heavy numeric treatment, small cyan metadata, and condensed hierarchy reproduce the supplied dashboard's character while remaining legible at the 1080 × 640 minimum window.
- Spacing and layout rhythm: glass cards, rounded corners, cyan edge light, compact headers, ring/secondary-metric rhythm, and CTA placement are consistent. The four-card implementation now directly matches the reference's 2 × 2 composition. One, two, and three-card pages use 1 × 1, 2 × 1, and 1 × 3 respectively.
- Colors and visual tokens: deep navy surfaces, cyan/mint global states, orange Claude, purple Antigravity, blue Copilot, restrained translucent borders, and glow intensity map to the reference.
- Image quality and asset fidelity: the supplied AgentMeter symbol is integrated as a transparent raster asset; generated provider and empty-state PNG assets match the source art direction without checkerboard artifacts, text substitutes, inline SVG, or CSS illustration stand-ins.
- Copy and content: all visible copy is coherent Traditional Chinese, retains the product's real quota/status semantics, and distinguishes unknown from zero.
- Accessibility and responsiveness: meaningful buttons remain keyboard focusable, status text is preserved for assistive technology, target sizes remain usable, and no document overflow or card-content clipping was found across the verified desktop/tablet/phone viewports.

**Comparison history**

1. Initial comparison found three actionable differences: a P1 checkerboard baked into the generated wordmark, a P2 percentage numeral overflow on narrow phones, and P2 undersized gauges in tall desktop cards. Fixes: replaced the wordmark bitmap with the transparent supplied symbol plus native text, constrained responsive numeral sizing, and added comfortable-density gauge scaling. Post-fix evidence: `D:\WorkSpace\AgentMeter\.scratch\desktop-adaptive.png` and the adaptive browser verifier.
2. Final same-size full-view and focused-card comparisons found no remaining actionable P0/P1/P2 issue. The generated cloud/slash empty-state illustration was added in the final pass to close the remaining asset-level difference.
3. User feedback exposed a P1 product-flow mismatch: the dashboard add tile consumed primary monitoring space and monitor management belonged in settings. It also exposed a P1 topology mismatch for odd/even card counts. The add tile was removed, add/remove/reorder moved into settings, and the exact 1/2/3/4 matrix plus four-item pagination was locked with tests.
4. Geometry verification found P2 clipping in desktop settings at 1080 × 640 and 1440 × 900, plus a P2 three-card unknown-state overflow at 844 × 390. Adaptive settings paging and compact landscape card states resolved all three cases.
5. The supplied ultra-wide screenshot exposed a P1 clipped primary quota ring. The card's flex column compressed `.quota` below the combined minimum height of its primary and weekly windows; `.quota { overflow: hidden }` then cut the centered primary window and ring. The fix caps the comfortable ring/window pair by viewport height, reserves the full glow extent at wide four-card sizes, and switches dense four-card views to compact mode sooner. Post-fix browser geometry contains every quota window and the computed ring paint rectangle at 2491 × 1312, 1993 × 1050, and the existing desktop matrix.
6. The follow-up 2440 × 1288 screenshot exposed a P1 percentage-to-gauge scale mismatch: the 64px value occupied 106–108% of the transparent ring center. Responsive numeral caps now follow the ring breakpoint. The browser gate measures only the visible percentage text and requires it to fit within 90% of the ring's 69% inner diameter; `8%`, `47%`, `82%`, and worst-case `100%` states pass.

**Browser-rendered evidence**

- Desktop and tablet previews were opened in the in-app browser.
- Primary interactions tested: desktop settings add/remove/reorder, the three-card vertical composition, restoration to the four-card 2 × 2 composition, tablet connection dialog, provider refresh/setup controls, tablet dashboard, and tablet settings.
- Browser console errors checked: none on desktop or tablet.
- Adaptive verification: 9 desktop plus 7 tablet/phone viewports; exact 1/2/3/4 layouts; 12-monitor pagination in groups of four; zero add tiles; zero document scroll; quota windows, wide four-card ring paint, and primary percentage text fully contained.
- Settings evidence: `D:\WorkSpace\AgentMeter\.scratch\desktop-settings.png` and `D:\WorkSpace\AgentMeter\.scratch\tablet-settings.png`.

**Open Questions**

- None blocking. Authenticated live-provider appearance, a physical tablet, and clean-VM installer rendering remain environment acceptance checks rather than design mismatches.

**Implementation Checklist**

- [x] Integrate brand, provider, and empty-state assets.
- [x] Match the supplied glass dashboard visual system across every existing desktop and tablet surface.
- [x] Preserve product behavior, unknown-state semantics, pairing/security boundaries, and adaptive pagination.
- [x] Keep monitor membership and ordering in settings, with honest catalog-only Cursor/Kiro states.
- [x] Keep primary quota rings fully visible at ultra-wide and high-DPI-equivalent desktop sizes.
- [x] Keep primary percentages, including 100%, inside the gauge's clear center with breathing room.
- [x] Verify source-to-implementation visual fidelity and browser interactions.

**Follow-up Polish**

- P3 only: when Cursor or Kiro collectors are implemented, promote their existing catalog rows to selectable monitors without auto-selecting them for current users.

final result: passed
