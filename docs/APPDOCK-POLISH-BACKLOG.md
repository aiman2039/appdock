# AppDock polish implementation backlog

This backlog implements the direction in [AppDock interaction and window cohesion](APPDOCK-POLISH-RESEARCH.md). The research baseline is AppDock 0.12.0, commit `237731c3ffbeded3a7463ec391acef07a3d3d834`; 92 deterministic tests passed on September 14, 2026. Native acceptance remains separate.

## Accepted scope: high-confidence improvements only

Decision recorded September 14, 2026: preserve all research and confidence assessments, and implement only the high-confidence items for now. High confidence describes expected benefit, not proof from before/after UX testing.

- **Implemented locally:** skip redundant checked resize on already docked, freshly verified matching windows during tab switching (the narrow part of P0-03).
- **Implemented locally:** per-tab minimize state, an opaque Restore surface, and continued use of healthy tabs, preserving existing dialog/fullscreen/permission safeguards (the narrow part of P0-04/P0-06).
- **Implemented locally:** clearer minimized-window picker actions and attachment progress using existing discovery and restore capability checks (the high-confidence feedback portion of P0-05).
- **Deferred:** selected-window-first drag scheduling, external-title-bar following, command-lane changes, deferred inactive movement, observer/capability expansion, capture, private APIs, automatic anti-minimize recovery, and the remaining P1/P2 work.

The proposals below remain unimplemented unless an implementation/validation record explicitly says otherwise. They are retained for later consideration, not authorization to broaden this pass.

See [implementation and validation](HIGH-CONFIDENCE-POLISH.md): all required local checks and 100 Rust tests pass; six focused native cases pass using disposable windows. Failures, the native transition interval, and real-app validation limits are recorded there. The broader P0-03/P0-04/P0-05/P0-06 scopes below are not completed by this narrow pass.

Priority means impact and dependency, not a calendar estimate. P0 establishes trustworthy movement and state; P1 completes interaction and lifecycle behavior; P2 evaluates optional architecture experiments.

## P0-01 — Record a reproducible interaction baseline

**Problem:** existing movement timing measures AX calls, and tracking fixtures measure timer execution. Neither quantifies the visible separation of the manager and target.

**Scope:** add opt-in movement telemetry and extend the disposable native harness. Record gesture/revision, queue age, AX duration, actual selected frame, manager frame, cover bounds, and errors. Use session-local IDs in export; exclude titles, document paths, and captured user content by default.

**Acceptance:** repeated traces with 1/3/8/20 windows, same-app and mixed-app tabs, deliberate inactive-target delay, physical dragging, direction reversals, and available 60/120 Hz displays. Export sample count, percentiles, failures, binary hash, OS and app versions. Pair timing with visual playback. Never describe WindowServer readback as direct proof of compositor presentation.

**Files:** `src/ui.rs`, `src/worker.rs`, `src/engine.rs`, `src/macos.rs`, `src/ui/fixtures_review.rs`, `scripts/review-native.py`.

## P0-02 — Move the selected window first

**Problem:** `follow_workspace` moves a HashMap's docked members sequentially, without selected priority, and returns on the first failure.

**Scope:** stable selected-first ordering; revision and lifecycle checks between writes; local handling of inactive failures; preservation of latest geometry and original snapshots. Keep moving inactive windows for this first patch so the current cover invariant survives.

**Meaningful regressions:** slow first inactive member; inactive failure followed by a healthy selected member; newer geometry arriving mid-loop; release/pause/selection during movement; a window with a larger accepted size; pointer-up with no further move notification. Assert actual mutation order and that obsolete or released targets receive no later writes.

**Acceptance:** selected latency no longer includes preceding inactive writes in the same batch. Explicitly report remaining latency from a background call already in flight. No new inactive-window exposure or pointer-routing failures. Do not claim request isolation until it is implemented.

**Depends on:** P0-01. **Files:** `engine.rs`, `worker.rs`, `macos.rs`.

## P0-03 — Remove avoidable settling from ready tab switches

**Problem:** every switch invokes checked `set_frame`, even when the target is already at the required geometry. Its stability polling introduces waits before focus.

**Scope:** reuse a freshly validated matching frame; keep checked geometry for stale, constrained, restored, or mismatched targets. Add pending feedback immediately and retain confirmed-selection semantics. Introduce interactive operation budgets and prevent discovery from starting new reads when movement is pending.

**Meaningful regressions:** ready target skips geometry writes; changed frame still settles; stale generation cannot confirm selection; modal/fullscreen interruption preserves the prior valid state; rename/picker barrier wins against in-flight focus work.

**Acceptance:** compare ready-target switching before/after. Retain full rollback and minimized-window readiness behavior. A queued priority command must stop background work at the next safe checkpoint.

**Depends on:** P0-01. **Files:** `engine.rs`, `native_ops.rs`, `worker.rs`, `ui.rs`.

## P0-04 — Introduce lifecycle state with explicit scope

**Problem:** one string-valued global pause handles unrelated cases, including an inactive window's minimization.

**Scope:** separate workspace permission/session suspension, application modal/unavailable state, and window visibility/readiness. Keep selection, ownership, focus intent, original restoration state, and movement phase distinct. Use orthogonal fields where conditions coexist rather than one enormous mutually exclusive enum.

**Transitions to define:** minimized/restored, app hidden/unhidden, window destroyed, AX handle replaced, modal opened/closed, native fullscreen entered/exited, display/Space changed, permission revoked, release begun, restore failed, and recovery completed.

**Acceptance:** a local interruption does not disable healthy unrelated apps. App-modal conditions protect all members of that process. Unknown modal scope prevents unsafe writes. Permission loss still suspends all control. Recovery snapshots never disappear because a presentation state changed.

**Depends on:** P0-01. **Files:** `model.rs`, `engine.rs`, `worker.rs`, `ui.rs`.

## P0-05 — Make minimized-window discovery resilient

**Problem:** candidate booleans do not explain uncertainty, some scan failures clear all results, and basic eligibility requires minimization support even for visible windows.

**Scope:** state confidence and rejection reasons; per-app scan outcomes; incremental successful results; stale-result labels; refresh on lifecycle events with polling fallback. Preserve merged AXWindows/AXChildren and exact identity. Decouple visible-window docking from optional minimize capability.

**Meaningful regressions:** minimized AXDialog; delayed AXMain; windows only in AXChildren; unsupported AXMinimized versus permission failure; one hung app among healthy apps; duplicate titles; window disappears while selected; minimized state changes during Add; successful but empty source versus failed source.

**Acceptance:** “Restore & Add” reflects the selected candidate. Scanning never changes minimization or focus. Selection is revalidated before a mutation; stale identities never select a different window. Unsupported notification registration activates a bounded fallback without removing otherwise valid candidates.

**Depends on:** P0-04. **Files:** `macos.rs`, `model.rs`, `native_ops.rs`, `worker.rs`, `picker.rs`, `ui.rs`.

## P0-06 — Recover minimized tabs without pausing the group

**Scope:** an inactive minimized tab remains attached but stops movement. A selected minimized tab receives an opaque local Restore surface; other tabs remain usable. Explicit selection restores once, revalidates, positions, focuses, and confirms. Failed restore remains local and releasable.

**Meaningful regressions:** inactive minimize during drag; selected minimize during switch; unsupported/missed observer event; restoration callback arrives after release; minimize followed by app hide; minimize while manager hidden; partial restore and retry; user repeats minimize; original minimized state still restores on explicit release.

**Acceptance:** no workspace-wide pause from an ordinary minimize event. No desktop or inactive app visible through a transparent placeholder. No automatic focus stealing from background recovery. Replace the old `d2_minimizing_one_window_pauses_all_docking_until_full_resume` contract with scoped assertions.

**Optional follow-up:** a bounded “Keep selected window open” setting, clearly described as recovery, with recursion suppression and cancellation. It is not part of the default behavior until tested.

**Depends on:** P0-04, P0-05. **Files:** `engine.rs`, `worker.rs`, `ui.rs`, `backdrop.rs`.

## P1-01 — Prototype movement led by the active app

**Scope:** introduce a movement coordinator with one leader, gesture generation, requested/confirmed frames, settling, and interruption. AppDock-header gestures remain the reliable entry point. Test external-title-bar following separately before replacing snap-back.

**Acceptance:** AX echoes never create a feedback loop; app-initiated layout changes do not become user drags; content/file drags are unchanged; rapid selection invalidates prior movement; mouse-up reconciles final geometry. Escaping a tab drag cancels its own reordering without moving native content unexpectedly.

**Decision gate:** ship only if external drag classification is reliable across native, Electron, and Java apps. Otherwise retain the explicit group drag region and document the external-title-bar limitation.

**Depends on:** P0-02, P0-04. **Files:** `ui.rs`, `engine.rs`, `worker.rs`, `window_tracking.rs`.

## P1-02 — Isolate slow apps and prove inactive coverage

**Scope:** use P0-01 traces to determine whether per-process command lanes are needed. Maintain per-window ownership and process-serial focus/write ordering. Consider deferred inactive updates only with a demonstrated old/new cover strategy.

**Acceptance:** hung inactive apps cannot monopolize healthy selected-app movement. Detached objects receive no further writes. No visible inactive content or shadow trails, broad blank cover, blocked desktop clicks, or leaked cover after crash/close. Preserve normal window levels and exact stacking anchors.

**Decision gate:** reject deferred movement if coverage is worse than moving all members. Never “fix” the lag by leaving uncovered inactive windows behind.

**Depends on:** P0-02, P0-04, P0-06. **Files:** `worker.rs`, `macos.rs`, `engine.rs`, `backdrop.rs`.

## P1-03 — Finish the tab strip and focus transitions

**Scope:** stable header height; clear pending/active/minimized states; title disambiguation for same-app windows; insertion preview, edge autoscroll, Escape, and keyboard reorder. Keep release and native close distinct. Select a healthy neighbor after active release/closure.

**Acceptance:** no geometry jump for routine status messages; no accidental app focus while editing; first click and scroll hit the intended target; no stolen Cmd+W/Cmd+M; VoiceOver identifies selection and release action. Verify IME, clipboard, context menu, and file-drop behavior after repeated switches.

**Depends on:** P0-03, P0-06. **Files:** `ui.rs`, `picker.rs`, `appearance.rs`, native fixtures.

## P1-04 — Complete display and workspace lifecycle

**Scope:** explicit session sleep/wake, display removal, app hide/unhide, Space changes, fullscreen, modal, and manager-minimize transitions. If adding Minimize Workspace, use a separate transaction recording only the windows it changes.

**Acceptance:** no guessing closure from invisibility; monitor unplug leaves recovery on an available display; unrelated same-app windows stay untouched; group restore never restores a window that was already minimized beforehand. Recovery respects native modal loops and preserves pending original-state restoration.

**Depends on:** P0-04, P0-06. **Files:** `ui.rs`, `model.rs`, `engine.rs`, `worker.rs`.

## P1-05 — Add a recovery journal and sustained acceptance

**Scope:** minimal journal separate from normal workspace preferences, persisted before first external mutation. Store original recovery requirements and sufficient validated identity; never trust recycled PIDs/window numbers alone. Journal failures must stop unsafe mutation or leave an explicit unresolved record.

**Acceptance:** disposable-process crash injection at attach, switch, drag, release, partial restore, and quit. No automatic title-based rebinding. Ambiguous candidates require explicit selection. Successful recovery clears only its completed records. Existing “fresh launch clears old tabs” behavior remains compatible.

Run the report's app/display matrix, a repeated focus-switch loop, and a full working-day soak. Retain every failure and intermittent result. Release acceptance requires visual and native evidence in addition to deterministic tests.

**Depends on:** P0-04. **Files:** `persistence.rs`, `model.rs`, `engine.rs`, `worker.rs`, fixtures and validation docs.

## P2 — Evaluate remaining architecture gaps

**Capture experiment:** optional ScreenCaptureKit drag preview. Prove permission-denied fallback, stale/minimized content, popup arrival, dual-cover behavior, and seamless return to real-window input. No permanent capture requirement or synthetic input in the default design.

**Private-API experiment:** establish exact ownership privileges and OS compatibility for the proposed operation. A JankyBorders transaction declaration does not prove foreign-window grouping. Require a public fallback and separate packaging decision; no reduced system protection for the default product.

**Cooperating-app integration:** a limited native integration can support stronger composition and control policies when the target explicitly participates. Evaluate only with a concrete supported app and interaction contract.

**Depends on:** measured defects remaining after P0/P1. Success requires a better recorded user experience, not merely a working API call.

## Definition of completion

The user can add a minimized app, restore it, switch tabs rapidly, drag and resize the group, handle dialogs, minimize one member, and release/quit without losing control of any window. Healthy tabs continue when another app is unavailable. The visible frame stays convincing under real dragging, not just in a screenshot. Native input remains native, recovery is understandable, and validation states exactly which apps and systems were exercised.
