//! Public WindowServer geometry and transient stacking anchors. AX owns identity.
use crate::model::Rect;
use objc2_core_foundation::{
    CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGRect,
};
use objc2_core_graphics::{
    CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo, CGWindowListOption,
    kCGWindowBounds, kCGWindowLayer, kCGWindowNumber, kCGWindowOwnerPID,
};
pub fn frame(window_number: isize) -> Option<Rect> {
    let id = u32::try_from(window_number).ok()?;
    let windows = CGWindowListCopyWindowInfo(CGWindowListOption::OptionIncludingWindow, id)?;
    // CGWindowListCopyWindowInfo documents an array of CF dictionaries.
    let windows = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(windows) };
    let info = windows.get(0)?.downcast::<CFDictionary>().ok()?;
    let info = unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(info) };
    let bounds = info
        .get(unsafe { kCGWindowBounds })?
        .downcast::<CFDictionary>()
        .ok()?;
    let mut r = CGRect::default();
    if !unsafe { CGRectMakeWithDictionaryRepresentation(Some(&bounds), &mut r) } {
        return None;
    }
    Some(Rect {
        x: r.origin.x,
        y: r.origin.y,
        width: r.size.width,
        height: r.size.height,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct StackWindow {
    pub number: u32,
    pub pid: i32,
    pub frame: Rect,
}

/// Normal onscreen windows, in front-to-back order. No titles or image capture.
pub fn stack() -> Option<Vec<StackWindow>> {
    let list = CGWindowListCopyWindowInfo(
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        0,
    )?;
    let list = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(list) };
    Some(
        list.iter()
            .filter_map(|entry| {
                let dict = entry.downcast::<CFDictionary>().ok()?;
                let dict =
                    unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(dict) };
                let number = |key: &CFString| dict.get(key)?.downcast::<CFNumber>().ok()?.as_i64();
                if number(unsafe { kCGWindowLayer })? != 0 {
                    return None;
                }
                let bounds = dict
                    .get(unsafe { kCGWindowBounds })?
                    .downcast::<CFDictionary>()
                    .ok()?;
                let mut r = CGRect::default();
                if !unsafe { CGRectMakeWithDictionaryRepresentation(Some(&bounds), &mut r) } {
                    return None;
                }
                Some(StackWindow {
                    number: u32::try_from(number(unsafe { kCGWindowNumber })?).ok()?,
                    pid: i32::try_from(number(unsafe { kCGWindowOwnerPID })?).ok()?,
                    frame: Rect {
                        x: r.origin.x,
                        y: r.origin.y,
                        width: r.size.width,
                        height: r.size.height,
                    },
                })
            })
            .collect(),
    )
}

/// Called only after AXRaise and exact AXFocusedWindow readback. The frontmost
/// normal window of that process must also match its AX frame; never guess by title.
pub fn focused_number(pid: i32, frame: Rect) -> Option<u32> {
    let window = stack()?.into_iter().find(|w| w.pid == pid)?;
    window.frame.near(frame).then_some(window.number)
}

/// WindowServer numbers for the workspace group. Fully destructured by callers.
pub struct RevealWindows {
    pub manager: u32,
    pub backdrop: Option<u32>,
    pub selected: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealAction {
    /// No selected window on this Space; show AppDock alone.
    ShowManager,
    /// Selected is unburied; AppDock should sit immediately below it.
    Tuck,
    /// An unrelated window is above the selected docked window.
    RaiseThenTuck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealProgress {
    Idle,
    Wait,
    Tuck,
    ShowManager,
}

/// Front-to-back stack. Manager and backdrop above the selected window are ours.
pub fn reveal_action(stack: &[StackWindow], windows: RevealWindows) -> RevealAction {
    let RevealWindows {
        manager,
        backdrop,
        selected,
    } = windows;
    let Some(selected) = selected else {
        return RevealAction::ShowManager;
    };
    let Some(selected_index) = stack.iter().position(|w| w.number == selected) else {
        return RevealAction::ShowManager;
    };
    let ours = |number: u32| number == manager || backdrop == Some(number);
    let buried = stack[..selected_index]
        .iter()
        .any(|window| !ours(window.number));
    if buried {
        RevealAction::RaiseThenTuck
    } else {
        RevealAction::Tuck
    }
}

pub fn reveal_progress(pending: bool, timed_out: bool, action: RevealAction) -> RevealProgress {
    if !pending {
        return RevealProgress::Idle;
    }
    match action {
        RevealAction::Tuck => RevealProgress::Tuck,
        RevealAction::ShowManager => RevealProgress::ShowManager,
        RevealAction::RaiseThenTuck if timed_out => RevealProgress::ShowManager,
        RevealAction::RaiseThenTuck => RevealProgress::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(number: u32) -> StackWindow {
        StackWindow {
            number,
            pid: 1,
            frame: Rect::default(),
        }
    }

    fn windows(selected: Option<u32>) -> RevealWindows {
        RevealWindows {
            manager: 10,
            backdrop: Some(11),
            selected,
        }
    }

    #[test]
    fn covering_window_above_selected_requires_raise() {
        let stack = [w(99), w(20), w(10), w(11)];
        assert_eq!(
            reveal_action(&stack, windows(Some(20))),
            RevealAction::RaiseThenTuck
        );
    }

    #[test]
    fn selected_already_front_tucks_manager() {
        let stack = [w(20), w(10), w(11), w(99)];
        assert_eq!(reveal_action(&stack, windows(Some(20))), RevealAction::Tuck);
    }

    #[test]
    fn manager_raised_above_cover_is_still_buried() {
        let stack = [w(10), w(99), w(20), w(11)];
        assert_eq!(
            reveal_action(&stack, windows(Some(20))),
            RevealAction::RaiseThenTuck
        );
    }

    #[test]
    fn only_our_windows_above_selected_is_tuck() {
        let stack = [w(10), w(11), w(20), w(99)];
        assert_eq!(reveal_action(&stack, windows(Some(20))), RevealAction::Tuck);
    }

    #[test]
    fn missing_or_empty_selection_shows_manager() {
        let stack = [w(99), w(10)];
        assert_eq!(
            reveal_action(&stack, windows(None)),
            RevealAction::ShowManager
        );
        assert_eq!(
            reveal_action(&stack, windows(Some(20))),
            RevealAction::ShowManager
        );
        assert_eq!(
            reveal_action(&[], windows(Some(20))),
            RevealAction::ShowManager
        );
    }

    #[test]
    fn reveal_progress_waits_until_unburied_or_timeout() {
        assert_eq!(
            reveal_progress(false, false, RevealAction::RaiseThenTuck),
            RevealProgress::Idle
        );
        assert_eq!(
            reveal_progress(true, false, RevealAction::RaiseThenTuck),
            RevealProgress::Wait
        );
        assert_eq!(
            reveal_progress(true, true, RevealAction::RaiseThenTuck),
            RevealProgress::ShowManager
        );
        assert_eq!(
            reveal_progress(true, false, RevealAction::Tuck),
            RevealProgress::Tuck
        );
        assert_eq!(
            reveal_progress(true, false, RevealAction::ShowManager),
            RevealProgress::ShowManager
        );
    }
}
