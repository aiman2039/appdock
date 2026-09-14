# Tab overflow and clipped height

Implemented locally on September 14, 2026 in response to the crowded, vertically clipped tab strip shown after adding an app.

The old tab document was 32 points tall inside a 34-point scroll view with an automatically appearing horizontal scrollbar. A native reproduction using a legacy scrollbar reduced the visible viewport from **34 to 17 points** when the document began to overflow. This reproduces a concrete cause of the clipped tabs in the supplied image. [Native reproduction and regression log](review-evidence/tab-overflow/tabs.log).

## Changes

- Left and right arrows appear when the tabs exceed the available width. Each click moves one tab width; the corresponding arrow disables at each end. Both arrows disappear when the tabs fit.
- The strip remains scrollable using horizontal trackpad gestures or a mouse wheel, with no native scrollbar. Scroll input over native app content is unaffected.
- The tab document and viewport stay 32 points tall. Adding tabs does not consume vertical space or move the app area. The scroll offset is clamped horizontally with a zero vertical offset.
- Newly selected and renamed tabs are brought into view. Normal refreshes retain manual horizontal browsing instead of repeatedly returning to the selected tab.
- Removing tabs or widening the window clamps the scroll offset and updates the arrows.

The native controls are implemented in [`tab_scroll.rs`](../src/ui/tab_scroll.rs) and integrated into the existing [`ui.rs`](../src/ui.rs) layout. Wheel handling follows Apple's distinction between [precise and coarse scrolling deltas](https://developer.apple.com/documentation/appkit/nsevent/hasprecisescrollingdeltas), using [scrollingDeltaX](https://developer.apple.com/documentation/appkit/nsevent/scrollingdeltax) and its vertical counterpart.

## Validation

`./scripts/check.sh` passed: metadata checks, formatting, compilation, strict Clippy, all **100 Rust tests**, and the locked build. [Check log](review-evidence/tab-overflow/checks.log).

The [native manifest](review-evidence/tab-overflow/results.json) records four passing cases on macOS 26.6.2 / Apple Silicon, with the tested binary hash:

- **tabs:** reproduces the old clipping; renders 3 then 12 tabs through the real UI; verifies unchanged workspace/app-area geometry; presses both arrow buttons; checks end limits; delivers local trackpad and mouse-wheel events; verifies selection reveal, retained manual scrolling, full-height offscreen rename, tab removal, and window resizing.
- **frame:** integrated attachment, focus, rename, switching, manager geometry, release, and close.
- **pointer:** app-body and tab-strip click routing.
- **polish:** per-tab minimization and Restore remain functional.

The [overflow preview](review-evidence/tab-overflow/tabs.png) was inspected. It uses disconnected dummy tabs and an internal view bitmap; it is not a capture of the user's workspace. Native mutations in the other cases used disposable child windows and fresh temporary data directories. Existing backend fixes and unrelated dirty files were preserved byte-for-byte.

This fixes the demonstrated tab-row clipping. The cropped image does not independently establish a separate full-workspace height problem caused by an application's size constraints. The installed bundle and a physical trackpad session in the user's running workspace were not exercised or replaced.

After a local build, repeat with:

```sh
python3 scripts/review-native.py --output /tmp/appdock-tab-review --cases tabs,frame,pointer,polish
```
