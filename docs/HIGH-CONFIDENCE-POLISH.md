# High-confidence AppDock improvements

Implemented locally on September 14, 2026. This is the narrow scope accepted after the [research](APPDOCK-POLISH-RESEARCH.md) and confidence review. It does not implement the deferred drag, capture, private-API, or broader discovery experiments.

## Behavior

- **Ready tab switching:** a previously docked window whose freshly read frame matches both its accepted frame and the current docking area skips the resize/settling call. Focus is still required. Restored, moved, newly attached, or mismatching windows continue through checked positioning. No numerical real-app speedup is claimed.
- **Minimized tabs:** minimization retains tab membership without globally pausing docking. Observed minimized windows are excluded from movement and cover geometry. A selected minimized tab displays an opaque owned Restore surface; other tabs remain selectable.
- **Restore:** the button requests the existing restore/readiness/position/focus sequence and disables repeated submission while pending. Explicit tab selection also restores. Original restoration snapshots survive failures. Readiness failures remain local and wait for explicit retry; successful restoration clears the local state.
- **External restore:** a window restored through macOS is revalidated before automatic redocking. Polling does not request focus. Cancelled validation can retry after editing ends; permission failure retains global suspension.
- **Resume:** global Resume aligns healthy windows while leaving minimized members minimized until explicitly selected. Dialog, fullscreen, and permission safeguards remain in place.
- **Picker feedback:** minimized candidates use **Restore & Add**. The label updates when the selected candidate's state changes, and a disappearing candidate disables attachment. Pending attachment/switch work is published for tab feedback without adding a separate progress row that changes the docking area.

Discovery sources, eligibility requirements, AX timeout policy, polling schedule, ordinary drag ordering, release semantics, and startup rules retain their existing design. Expanded compatibility, observer integration, and per-app partial discovery are deferred.

## Verification

The repository's complete `./scripts/check.sh` passes: three update-metadata checks, formatting, all-target/all-feature compilation, strict Clippy, **100 Rust tests**, and the locked build. The prior baseline was 92 Rust tests. New regressions cover the ready-switch shortcut, mismatched geometry, minimized restoration validation, local minimize state, healthy-tab movement/switching, original snapshot retention, explicit retry after failed readiness, global Resume, and preservation of modal/fullscreen/permission safeguards. [Check log](review-evidence/high-confidence-polish/checks.log).

The [final native manifest](review-evidence/high-confidence-polish/final/results.json) records successful prerequisites and six passing cases on macOS 26.6.2 / Apple Silicon:

| Case | Verified behavior |
| --- | --- |
| `polish` | Selected minimize; opaque and clickable recovery surface; actual Restore button; disabled repeated restore; picker action changes; switching/moving/resizing a healthy tab while another remains minimized; explicit selection restores; close completes |
| `Minimized` | Fresh discovery of already-minimized disposable windows, attachment, and restoration of original minimized state on release |
| `RestoreReady` | Delayed native AXMain readiness after restoration |
| `pointer` | External app-body and AppDock tab-strip click routing after manager movement/resize |
| `A4` | Global Resume preserves a minimized member, aligns healthy members, leaves never-docked attachments untouched, and explicit selection restores the minimized member |
| `frame` | Integrated picker, settings, rename focus, repeated switching, frame fitting, direct activation, movement/resize, release, and close |

The [recovery bitmap](review-evidence/high-confidence-polish/final/polish.png) was inspected. It verifies the recovery surface; its tab strip is an intermediate fixture rendering, not a product screenshot or evidence of smooth dragging.

### Retained failures and limits

The [initial native run](review-evidence/high-confidence-polish/initial/results.json) caught a pointer-routing failure during the selected-window minimize transition. The manager now explicitly becomes opaque for recovery and returns to transparency for native content. The fixture waits a bounded interval for native minimization and ordering to settle. In the final log, the first transition sample still hit a sibling window, then converged; this does not claim instantaneous or animation-free minimization.

The old native A4 assertion expected workspace-wide pause and was replaced with the newly accepted per-tab contract. The [intermediate rerun](review-evidence/high-confidence-polish/recheck/results.json) passed Restore interactions but encountered an unrelated desktop window during a later app-body pointer check. The fixture now explicitly reacquires its disposable target after manager resizing before checking app-body routing. The final integrated frame and pointer cases also passed. Failed logs remain available rather than being replaced with passing runs.

All native mutations targeted disposable child windows and fresh temporary workspaces. Existing user windows were not attached or changed. No installed bundle, release, hosted CI run, or deployment was updated. The unrelated workflow and development-guide edits present before this work were preserved byte-for-byte.

Detection still uses polling and depends on target responsiveness. The native results do not certify every application, monitor/Spaces arrangement, or extended desktop session. They also do not demonstrate the deferred Chrome-like drag behavior. Test the rebuilt app with the actual daily-use applications before treating the UX improvement as universally established.

To repeat the focused native coverage after building:

```sh
python3 scripts/review-native.py --output /tmp/appdock-polish-review --cases polish,Minimized,RestoreReady,pointer,A4,frame
```

Use an authorized native desktop launch context with Accessibility available; the runner reports missing prerequisites as failure and operates only on its disposable targets.
