# Plan: keep docked windows across Space/AX empty-list

SPEC Alignment: aligned — Space change pauses docking; empty AXWindows is not closure. Dropping handles on Missing then rebind `docked=false` made Resume a no-op.

Space change → Pause → AXWindows=[] → 3 Missing polls dropped handles → rebind attach() docked=false → Resume moved nothing.

```
Missing event
  → state() ok? keep handle, treat as Changed
  → paused? keep handle (do not count Missing)
  → else 3 misses → orphan snapshot, disconnect tab (keep saved tab)
Rebind → restore docked + original from orphan
Pause while already paused → do not reset 2s auto-resume timer
None window_number → do not block recovery
UI: Resume button + menu when paused
```

- [x] Task 1: Engine Missing/rebind — keep handle if state() ok or paused; orphan snapshot; rebind restores docked+original; tests
- [x] Task 2: Auto-resume — don't reset 2s timer on redundant Pause; None window_number does not block; tests
- [x] Task 3: Resume button in status row when paused

Out of scope: AXObserver.
Stay here.
