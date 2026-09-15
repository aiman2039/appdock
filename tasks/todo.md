# Plan: Dock click reveals workspace

SPEC Alignment: aligned — ARCHITECTURE.md stack is selected > AppDock > backdrop > inactive. Dock reopen never raises the selected foreign window, then tucks AppDock under it while it is still buried. No SPEC.md; product behavior from ARCHITECTURE.md + DEVELOPMENT.md.

Covering window in front → Dock click → AppDock `makeKeyAndOrderFront` (flicker) → tick `orderWindow(Below, selected)` while selected still behind cover → workspace stays hidden.

```
Dock click
  → applicationShouldHandleReopen
  → selected on-screen and buried?
       yes → Command::Raise, defer tuck until selected front (only our windows above it)
       no  → deminiaturize + makeKeyAndOrderFront (today)
  → selected front → order AppDock below selected; AppDock stays key
Title-bar / become-key: unchanged (immediate tuck, no AXRaise)
```

- [x] Task 1: `window_tracking` reveal predicate + unit tests
  - Input: front-to-back stack, manager#, backdrop#, selected#
  - Buried = unrelated window above selected (ignore our two windows)
  - Cases: buried, already front+tucked, selected missing, empty
- [x] Task 2: Dock reopen uses predicate
  - `reopen_window`: hidden/miniaturized still shows AppDock
  - Buried: `Command::Raise`; do **not** immediate `order_requested` tuck
  - Tick: tuck only after selected is front; timeout fallback shows AppDock
  - Title-bar path unchanged
  - AppDock remains key after tuck (menus); app body stays clickable
- [x] Task 3: Regression
  - Unit tests in Task 1 are the CI proof
  - Native: covering window + Dock-equivalent `reopen_window`; selected+chrome in front of cover
  - `scripts/check.sh`

Out of scope: Cmd+Tab/`DidBecomeActive`; raising inactive tabs; window levels; previous uncommitted Space/AX work.

Stay here.

## Unresolved

- Cmd+Tab when buried: same bug? (recommend follow-up)
- After Dock click, should keyboard stay on AppDock (recommended) or move to docked app?
