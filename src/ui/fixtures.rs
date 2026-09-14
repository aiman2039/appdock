//! Native fixture state and stage execution. AppKit callbacks may reenter:
//! release RefMut before focus, picker, rename, or window-order operations.
use super::*;
use crate::worker::Snapshot;
use std::cell::RefMut;
#[derive(Default)]
pub(super) struct FixtureState {
    pub(super) dock_test_stage: usize,
    pub(super) dock_test_tick: u64,
    pub(super) smoke_failed: bool,
    pub(super) fixture_child: Option<std::process::Child>,
    pub(super) fixture_rename_started: bool,
    pub(super) fixture_rename_tested: bool,
    pub(super) fixture_attach_pending: Option<WindowId>,
    pub(super) fixture_attach_after: u64,
    pub(super) fixture_picker_cancel_tested: bool,
    pub(super) fixture_reveal_started: bool,
    pub(super) fixture_reveal_tested: bool,
    pub(super) fixture_reveal_ordered: bool,
    pub(super) fixture_keyboard_ready: bool,
    pub(super) startup_menu_stage: u8,
    pub(super) settings_popup_stage: u8,
    pub(super) title_focus_tick: Option<u64>,
    pub(super) title_focus_tested: bool,
    pub(super) polish_stage: u8,
    pub(super) polish_selected: Option<TabId>,
    pub(super) polish_minimize: Option<std::thread::JoinHandle<Result<()>>>,
    pub(super) polish_issue_tick: Option<u64>,
}
impl Drop for FixtureState {
    fn drop(&mut self) {
        if let Some(mut child) = self.fixture_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl Delegate {
    pub(super) fn arm_settings_fixture(&self) {
        let testing = {
            let mut b = self.ivars().ui.borrow_mut();
            b.as_mut().is_some_and(|u| {
                if u.fixture.settings_popup_stage == 1 {
                    u.fixture.settings_popup_stage = 2;
                    true
                } else {
                    false
                }
            })
        };
        if !testing {
            return;
        }
        let timer = unsafe {
            NSTimer::timerWithTimeInterval_target_selector_userInfo_repeats(
                0.25,
                self,
                sel!(closeSettingsFixture:),
                None,
                false,
            )
        };
        unsafe {
            objc2_foundation::NSRunLoop::mainRunLoop()
                .addTimer_forMode(&timer, objc2_foundation::NSRunLoopCommonModes);
        }
    }
    pub(super) fn finish_settings_fixture(&self) {
        assert!(
            self.ivars().settings_tracking.get(),
            "Settings popup never entered native menu tracking"
        );
        let menu = self.ivars().startup_menu.get().unwrap();
        assert!(
            menu.indexOfItemWithTitle(&NSString::from_str("Save Current Apps for Startup")) >= 0
        );
        assert!(menu.indexOfItemWithTitle(&NSString::from_str("Reset Saved App Choices")) >= 0);
        let keep_open = menu.indexOfItemWithTitle(&NSString::from_str("Keep Apps Open on Close"));
        assert!(keep_open >= 0, "Keep-open preference missing from Settings");
        menu.performActionForItemAtIndex(keep_open);
        self.ivars()
            .ui
            .borrow_mut()
            .as_mut()
            .unwrap()
            .fixture
            .settings_popup_stage = 3;
        println!(
            "Visible Settings button opened the native menu after the focus barrier; Save and Reset actions are present"
        );
        menu.cancelTracking();
    }

    pub(super) fn fixture_picker_step<'a>(
        &self,
        count: u64,
        mut b: RefMut<'a, Option<Ui>>,
        _s: &Snapshot,
    ) -> Option<RefMut<'a, Option<Ui>>> {
        let u = b.as_mut()?;
        if u.picker_open
            && !u.pending_picker
            && count >= u.fixture.fixture_attach_after
            && let Some(id) = u.fixture.fixture_attach_pending.take()
        {
            if u.fixture.fixture_child.is_some() && !u.fixture.fixture_keyboard_ready {
                u.fixture.fixture_keyboard_ready = true;
                u.fixture.fixture_attach_pending = Some(id);
                u.fixture.fixture_attach_after = count + 2;
                drop(b);
                self.present_picker();
                return None;
            }
            u.fixture.fixture_keyboard_ready = false;
            let search = u.picker.search.clone();
            let test = u.fixture.fixture_child.is_some()
                && !std::env::args().any(|a| a == "--pointer-smoke");
            let cancel = test && !u.fixture.fixture_picker_cancel_tested;
            u.fixture.fixture_picker_cancel_tested |= test;
            if cancel {
                u.fixture.fixture_attach_pending = Some(id);
            }
            drop(b);
            if test {
                self.verify_rename_keyboard(&search);
                search.setStringValue(&NSString::from_str("__no_matching_window__"));
                self.filter();
                assert!(
                    self.ivars()
                        .ui
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .picker
                        .choice_ids
                        .is_empty(),
                    "Inline picker did not filter results"
                );
                search.setStringValue(&NSString::from_str(""));
                self.filter();
            }
            if cancel {
                self.dismiss_picker(true);
                assert!(
                    self.ivars()
                        .ui
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .picker
                        .view
                        .isHidden(),
                    "Cancel did not dismiss inline picker"
                );
                self.show_picker(false);
                return None;
            }
            assert!(
                self.ivars()
                    .ui
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .picker
                    .select_window(id),
                "Fixture target missing from inline picker"
            );
            self.attach_selected_window();
            println!("Integrated inline picker attachment submitted");
            return None;
        }
        Some(b)
    }
    pub(super) fn fixture_simple_step<'a>(
        &self,
        count: u64,
        mut b: RefMut<'a, Option<Ui>>,
        s: &Snapshot,
    ) -> Option<RefMut<'a, Option<Ui>>> {
        let u = b.as_mut()?;
        if std::env::args().any(|a| a == "--cleanup-smoke") && count == 12 {
            assert!(
                s.workspace.tabs.is_empty(),
                "Stale startup tabs were not removed"
            );
            assert!(
                s.live.is_empty() && s.selected.is_none(),
                "Fresh launch unexpectedly attached a window"
            );
            assert!(
                persistence::load(&persistence::path())
                    .unwrap()
                    .tabs
                    .is_empty(),
                "Startup cleanup was not persisted"
            );
            println!("Native startup cleanup passed: stale saved tabs removed and persisted");
            u.client.send(Command::Quit);
        }
        if std::env::args().any(|a| a == "--disconnected-smoke") {
            if count == 4 {
                drop(b);
                self.release_current();
                return None;
            }
            if count == 10 {
                assert!(
                    s.workspace.tabs.is_empty(),
                    "Disconnected tab was not removed"
                );
                println!("Disconnected tab release regression passed");
                u.client.send(Command::Quit);
            }
        }
        if std::env::args().any(|a| a == "--rename-smoke") {
            if count == 4 {
                let button = u
                    .tabs
                    .subviews()
                    .iter()
                    .find_map(|v| v.downcast::<TabButton>().ok())
                    .expect("Tab button missing");
                let number = u.window.windowNumber();
                drop(b);
                let event=NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(NSEventType::LeftMouseDown,NSPoint::new(20.,15.),NSEventModifierFlags::empty(),0.,number,None,1,2,1.).unwrap();
                unsafe {
                    let _: () = msg_send![&*button,mouseDown:&*event];
                }
                return None;
            }
            if count == 6 {
                let field = u
                    .rename_editor
                    .as_ref()
                    .expect("Double-click did not open inline rename")
                    .field
                    .clone();
                drop(b);
                self.verify_rename_keyboard(&field);
                field.setStringValue(&NSString::from_str("Discord - account 1"));
                self.finish_rename(true);
                return None;
            }
            if count == 10 {
                assert_eq!(s.workspace.tabs[0].name, "Discord - account 1");
                assert!(u.rename_editor.is_none());
                drop(b);
                self.begin_rename(7);
                return None;
            }
        }
        if std::env::args().any(|a| a == "--rename-smoke") {
            if count == 12 {
                let field = u.rename_editor.as_ref().unwrap().field.clone();
                drop(b);
                field.setStringValue(&NSString::from_str("Unsaved label"));
                self.finish_rename(false);
                return None;
            }
            if count == 16 {
                assert_eq!(s.workspace.tabs[0].name, "Discord - account 1");
                assert!(u.rename_editor.is_none());
                println!(
                    "Native rename regression passed: AppDock owns keyboard focus, Ctrl+A/Cmd+A select all, typing, commit and cancel"
                );
                u.client.send(Command::Quit);
            }
        }
        Some(b)
    }
    pub(super) fn fixture_docking_step<'a>(
        &self,
        count: u64,
        mut b: RefMut<'a, Option<Ui>>,
        s: &Snapshot,
    ) -> Option<RefMut<'a, Option<Ui>>> {
        let u = b.as_mut()?;
        if std::env::var_os("APPDOCK_POLISH_SMOKE").is_some()
            && u.fixture.dock_test_stage == 2
            && s.live.len() == 2
        {
            assert!(
                count <= 500,
                "Polish fixture timed out at stage {}: {}",
                u.fixture.polish_stage,
                s.status
            );
            let first = s.workspace.tabs[0].id;
            let second = s.workspace.tabs[1].id;
            if let Some(job) = &u.fixture.polish_minimize {
                if !job.is_finished() {
                    return None;
                }
                u.fixture
                    .polish_minimize
                    .take()
                    .unwrap()
                    .join()
                    .expect("Minimize worker panicked")
                    .expect("Native minimize failed");
            }
            match u.fixture.polish_stage {
                0 | 3
                    if s.preparing.is_none()
                        && (u.fixture.polish_stage == 0 || s.selected == Some(first))
                        && s.selected_frame.is_some_and(|frame| frame.near(s.area)) =>
                {
                    let id = s.selected.unwrap();
                    let number = s.backdrop.as_ref().unwrap().0;
                    let pid = u
                        .fixture
                        .fixture_child
                        .as_ref()
                        .expect("Disposable targets required")
                        .id() as i32;
                    u.fixture.polish_selected = Some(id);
                    u.fixture.polish_issue_tick = None;
                    u.fixture.polish_minimize = Some(std::thread::spawn(move || {
                        crate::macos::minimize_focused_fixture(pid, number)
                    }));
                    u.fixture.polish_stage += 1;
                    return None;
                }
                1 | 4 if s.selected_issue() == Some(&DockIssue::Minimized) => {
                    assert!(!s.paused);
                    assert_eq!(s.selected, u.fixture.polish_selected);
                    assert!(!u.recovery.isHidden(), "Minimized placeholder is missing");
                    assert!(
                        !u.surface.ivars().attached.get(),
                        "Minimized placeholder is transparent"
                    );
                    assert!(u.restore_button.isEnabled());
                    let center = u
                        .window
                        .convertRectToScreen(rect(u.surface.bounds().size.width / 2., 100., 1., 1.))
                        .origin;
                    let first_issue = *u.fixture.polish_issue_tick.get_or_insert(count);
                    let hit = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
                        center,
                        0,
                        self.mtm(),
                    );
                    if hit != u.window.windowNumber() && count - first_issue < 8 {
                        println!(
                            "Polish: waiting for native minimize/order transition, tick={}, hit={}, manager={}, members={:?}",
                            count - first_issue,
                            hit,
                            u.window.windowNumber(),
                            s.stacked_windows
                        );
                        return None;
                    }
                    assert_eq!(
                        hit,
                        u.window.windowNumber(),
                        "Inactive content intercepts the placeholder"
                    );
                    if u.fixture.polish_stage == 1 {
                        let bounds = u.surface.bounds();
                        let bitmap = u
                            .surface
                            .bitmapImageRepForCachingDisplayInRect(bounds)
                            .expect("Recovery bitmap unavailable");
                        u.surface
                            .cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
                        assert!(
                            bitmap
                                .colorAtX_y(bitmap.pixelsWide() / 2, bitmap.pixelsHigh() / 2)
                                .unwrap()
                                .alphaComponent()
                                > 0.99,
                            "Recovery content is not opaque"
                        );
                        if let Ok(output) = std::env::var("APPDOCK_SMOKE_SCREENSHOT") {
                            let data = unsafe {
                                bitmap.representationUsingType_properties(
                                    NSBitmapImageFileType::PNG,
                                    &objc2_foundation::NSDictionary::new(),
                                )
                            }
                            .expect("Recovery PNG encoding failed");
                            assert!(
                                data.writeToFile_atomically(&NSString::from_str(&output), true)
                            );
                        }
                        let mut candidate = s
                            .windows
                            .iter()
                            .find(|w| {
                                s.live
                                    .iter()
                                    .any(|(id, window)| Some(*id) == s.selected && *window == w.id)
                            })
                            .unwrap()
                            .clone();
                        candidate.minimized = true;
                        u.picker.render(self.mtm(), self, vec![&candidate], "");
                        assert_eq!(
                            u.picker.attach.attributedTitle().string().to_string(),
                            "Restore & Add"
                        );
                        candidate.minimized = false;
                        u.picker.render(self.mtm(), self, vec![&candidate], "");
                        assert_eq!(
                            u.picker.attach.attributedTitle().string().to_string(),
                            "Attach window"
                        );
                        u.picker
                            .render(self.mtm(), self, vec![], "Window no longer available");
                        assert!(!u.picker.attach.isEnabled());
                        let button = u.restore_button.clone();
                        u.fixture.polish_stage = 2;
                        drop(b);
                        unsafe {
                            button.performClick(None);
                        }
                        assert!(
                            !button.isEnabled(),
                            "Repeated restore submission is still enabled"
                        );
                        return None;
                    }
                    u.client.switch(second);
                    u.fixture.polish_stage = 5;
                    return None;
                }
                2 if s.selected_issue().is_none()
                    && s.preparing.is_none()
                    && s.selected_frame.is_some_and(|frame| frame.near(s.area)) =>
                {
                    self.verify_app_pointer_routes(
                        &u.window,
                        s.backdrop.as_ref().unwrap().0,
                        s.selected_frame.unwrap(),
                    );
                    assert!(u.recovery.isHidden());
                    println!(
                        "Polish: selected minimize kept workspace usable; opaque Restore surface, pointer routing, actual Restore button, and picker actions passed"
                    );
                    u.client.switch(first);
                    u.fixture.polish_stage = 3;
                    return None;
                }
                5 if s.selected == Some(second)
                    && s.preparing.is_none()
                    && s.selected_issue().is_none() =>
                {
                    assert!(!s.paused);
                    assert!(
                        s.issues
                            .iter()
                            .any(|(id, issue)| *id == first && *issue == DockIssue::Minimized)
                    );
                    let window = u.window.clone();
                    let mut frame = window.frame();
                    frame.origin.x += 35.;
                    frame.size.width += 60.;
                    u.fixture.polish_stage = 6;
                    u.fixture.dock_test_tick = count;
                    drop(b);
                    window.setFrame_display(frame, true);
                    return None;
                }
                6 if count > u.fixture.dock_test_tick + 10
                    && s.selected_frame.is_some_and(|frame| frame.near(s.area)) =>
                {
                    // AppKit frame changes are not focus requests. Explicitly
                    // reacquire our disposable child before the pointer assertion.
                    u.client.send(Command::Raise);
                    u.fixture.polish_stage = 8;
                    u.fixture.dock_test_tick = count;
                    return None;
                }
                8 if count > u.fixture.dock_test_tick + 3 && s.preparing.is_none() => {
                    assert!(!s.paused);
                    assert!(
                        s.issues
                            .iter()
                            .any(|(id, issue)| *id == first && *issue == DockIssue::Minimized)
                    );
                    self.verify_app_pointer_routes(
                        &u.window,
                        s.backdrop.as_ref().unwrap().0,
                        s.selected_frame.unwrap(),
                    );
                    println!(
                        "Polish: healthy tab switched, moved and resized while the inactive tab stayed minimized"
                    );
                    u.client.switch(first);
                    u.fixture.polish_stage = 7;
                    return None;
                }
                7 if s.selected == Some(first) && s.issues.is_empty() && s.preparing.is_none() => {
                    println!(
                        "Polish: explicit tab selection restored its window; closing restores original geometry"
                    );
                    u.client.send(Command::Quit);
                    u.fixture.dock_test_stage = 20;
                    return None;
                }
                _ => return None,
            }
        }
        if std::env::var_os("APPDOCK_SETTINGS_SMOKE").is_some() {
            if count == 2 {
                assert!(!u.settings_button.isHidden());
                assert!(u.settings_button.isEnabled());
                u.fixture.settings_popup_stage = 1;
                let button = u.settings_button.clone();
                drop(b);
                unsafe {
                    button.performClick(None);
                }
                return None;
            }
            if count == 19 {
                assert!(
                    !s.workspace.keep_apps_open_on_close,
                    "Settings did not honor the keep-open opt-out"
                );
                assert!(
                    !persistence::load(&persistence::path())
                        .unwrap()
                        .keep_apps_open_on_close,
                    "Keep-open opt-out was not persisted"
                );
                assert_eq!(
                    u.fixture.settings_popup_stage, 3,
                    "Settings popup did not complete"
                );
            }
        }
        if count == 8
            && std::env::args().any(|a| {
                matches!(
                    a.as_str(),
                    "--ui-smoke" | "--surface-smoke" | "--design-smoke"
                )
            })
        {
            let view = u.window.contentView().unwrap();
            let bounds = view.bounds();
            let bitmap = view
                .bitmapImageRepForCachingDisplayInRect(bounds)
                .expect("Native bitmap unavailable");
            view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
            if std::env::args().any(|a| a == "--surface-smoke") {
                assert!(
                    !u.window.hasShadow(),
                    "Docked controls still cast a shadow around the cutout"
                );
                let w = bitmap.pixelsWide();
                let h = bitmap.pixelsHigh();
                let alpha = |y| bitmap.colorAtX_y(w / 2, y).unwrap().alphaComponent();
                assert!(
                    alpha(h / 2) < 0.01,
                    "App area must not obscure the external window"
                );
                assert!(
                    alpha(20) > 0.99 || alpha(h - 20) > 0.99,
                    "Tab strip must stay opaque"
                );
                assert!(
                    bitmap.colorAtX_y(2, h / 2).unwrap().alphaComponent() > 0.99
                        && bitmap.colorAtX_y(w - 3, h / 2).unwrap().alphaComponent() > 0.99,
                    "AppDock side frame is missing"
                );
                assert!(
                    alpha(2) > 0.99 && alpha(h - 3) > 0.99,
                    "AppDock top/bottom frame is missing"
                );
                println!(
                    "Native surface regression passed: transparent app area, opaque controls and surrounding frame"
                );
            }
            let data = unsafe {
                bitmap.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &objc2_foundation::NSDictionary::new(),
                )
            }
            .expect("PNG encoding failed");
            let output =
                std::env::var("APPDOCK_SMOKE_SCREENSHOT").expect("Screenshot destination required");
            assert!(
                data.writeToFile_atomically(&NSString::from_str(&output), true),
                "Screenshot write failed"
            );
            println!("Native AppKit screenshot saved to {output}");
        }
        if std::env::args().any(|a| {
            matches!(
                a.as_str(),
                "--ui-smoke" | "--surface-smoke" | "--design-smoke"
            )
        }) && count
            == std::env::var("APPDOCK_SMOKE_TICKS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10)
        {
            u.client.send(Command::Quit);
        }
        if std::env::args().any(|a| {
            matches!(
                a.as_str(),
                "--dock-smoke" | "--picker-smoke" | "--frame-smoke" | "--pointer-smoke"
            )
        }) && u.fixture.dock_test_stage < 20
        {
            if s.status.starts_with("Paused:") || count > 400 {
                eprintln!(
                    "Integrated docking smoke failed at stage {}: {}",
                    u.fixture.dock_test_stage, s.status
                );
                u.fixture.smoke_failed = true;
                u.fixture.dock_test_stage = 20;
                u.client.send(Command::Quit);
            } else if count
                > u.fixture.dock_test_tick
                    + if u.fixture.fixture_child.is_some() {
                        2
                    } else {
                        8
                    }
            {
                let stage = u.fixture.dock_test_stage;
                if stage == 0 || stage == 1 {
                    let bundle = if stage == 0 {
                        "com.hnc.Discord"
                    } else {
                        "org.telegram.desktop"
                    };
                    let candidates: Vec<_> = s
                        .windows
                        .iter()
                        .filter(|w| {
                            w.eligible
                                && if let Some(child) = &u.fixture.fixture_child {
                                    w.pid == child.id() as i32 && !s.occupied.contains(&w.id)
                                } else {
                                    w.identity.bundle == bundle
                                }
                        })
                        .collect();
                    if (candidates.len() == 1
                        || (u.fixture.fixture_child.is_some() && !candidates.is_empty()))
                        && s.live.len() == stage
                        && (stage == 0 || s.selected == s.workspace.tabs.first().map(|t| t.id))
                    {
                        let window_id = candidates[0].id;
                        u.fixture.fixture_attach_pending = Some(window_id);
                        u.fixture.fixture_attach_after =
                            if stage == 0 && std::env::var_os("APPDOCK_PICKER_PREVIEW").is_some() {
                                count + 80
                            } else {
                                count
                            };
                        u.fixture.dock_test_stage += 1;
                        u.fixture.dock_test_tick = count;
                        let window = u.window.clone();
                        drop(b);
                        window.makeKeyAndOrderFront(None);
                        self.show_picker(false);
                        return None;
                    }
                } else if stage == 2
                    && std::env::args().any(|a| a == "--picker-smoke")
                    && s.live.len() == 2
                    && s.selected == s.workspace.tabs.last().map(|t| t.id)
                {
                    println!("Native picker regression complete; restoring windows");
                    u.client.send(Command::Quit);
                    u.fixture.dock_test_stage = 20;
                } else if (2..14).contains(&stage)
                    && s.live.len() == 2
                    && (stage == 2 || s.selected == Some(s.workspace.tabs[(stage - 3) % 2].id))
                {
                    if u.fixture.fixture_child.is_some() {
                        if !s.selected_frame.is_some_and(|frame| frame.near(s.area)) {
                            return None;
                        }
                        if std::env::args().any(|a| a == "--pointer-smoke") {
                            u.fixture.dock_test_stage = 14;
                            u.fixture.dock_test_tick = count;
                            return None;
                        }
                        assert!(
                            !u.window.hasShadow(),
                            "Docked workspace retained an extra shadow around its transparent cutout"
                        );
                        if !u.fixture.title_focus_tested {
                            if let Some(started) = u.fixture.title_focus_tick {
                                // Let activation and the delayed geometry/order work
                                // settle through several real UI/worker timer turns.
                                if count < started + 6 {
                                    return None;
                                }
                                let app = NSApplication::sharedApplication(self.mtm());
                                assert!(
                                    app.isActive(),
                                    "Title-bar activation returned focus to the docked app"
                                );
                                assert!(
                                    u.window.isKeyWindow(),
                                    "AppDock lost its key window after title-bar activation"
                                );
                                self.verify_app_pointer_routes(
                                    &u.window,
                                    s.backdrop.as_ref().unwrap().0,
                                    s.selected_frame.unwrap(),
                                );
                                let menu = app
                                    .mainMenu()
                                    .unwrap()
                                    .itemAtIndex(0)
                                    .unwrap()
                                    .submenu()
                                    .unwrap();
                                menu.update();
                                for title in ["Check for Updates…", "Settings…"] {
                                    let item = menu
                                        .itemWithTitle(&NSString::from_str(title))
                                        .expect("AppDock menu action missing");
                                    assert!(
                                        item.isEnabled(),
                                        "AppDock menu action is unavailable: {title}"
                                    );
                                }
                                u.fixture.title_focus_tested = true;
                                println!(
                                    "Title-bar activation retained AppDock keyboard focus and menu actions while the docked app remained clickable"
                                );
                            } else {
                                u.fixture.title_focus_tick = Some(count);
                                let window = u.window.clone();
                                drop(b);
                                // Exercise the same AppKit activation/key-window
                                // callbacks as clicking the native title bar. Also
                                // leave a geometry update pending, as after a drag.
                                #[allow(deprecated)]
                                NSApplication::sharedApplication(self.mtm())
                                    .activateIgnoringOtherApps(true);
                                window.makeKeyAndOrderFront(None);
                                let mut frame = window.frame();
                                frame.origin.x += 4.;
                                window.setFrame_display(frame, true);
                                return None;
                            }
                        }
                        if u.fixture.settings_popup_stage == 0 {
                            assert!(!u.settings_button.isHidden());
                            assert!(u.settings_button.isEnabled());
                            u.fixture.settings_popup_stage = 1;
                            let button = u.settings_button.clone();
                            drop(b);
                            unsafe {
                                button.performClick(None);
                            }
                            return None;
                        }
                        if u.fixture.settings_popup_stage < 3 {
                            return None;
                        }
                        if u.fixture.startup_menu_stage < 2 {
                            let mut menu_snapshot = s.clone();
                            let identity = &s.workspace.tabs[0].identity.bundle;
                            let bundle = if identity.is_empty() {
                                "dev.appdock.fixture"
                            } else {
                                identity.as_str()
                            };
                            // CLI fixture children have no app bundle. Supply their
                            // disposable identity to exercise the real menu action.
                            for window in &mut menu_snapshot.windows {
                                if u.fixture
                                    .fixture_child
                                    .as_ref()
                                    .is_some_and(|child| child.id() as i32 == window.pid)
                                {
                                    window.identity.bundle = bundle.to_owned();
                                }
                            }
                            self.update_startup_menu(&menu_snapshot);
                            let enabled = s
                                .workspace
                                .startup_apps
                                .iter()
                                .any(|app| app.bundle == bundle);
                            let stage = u.fixture.startup_menu_stage;
                            if (stage == 0 && !enabled) || (stage == 1 && enabled) {
                                let item = self
                                    .ivars()
                                    .startup_menu
                                    .get()
                                    .unwrap()
                                    .itemArray()
                                    .iter()
                                    .find(|item| {
                                        item.representedObject()
                                            .and_then(|o| o.downcast::<NSString>().ok())
                                            .is_some_and(|s| s.to_string() == bundle)
                                    })
                                    .expect("Startup app menu item missing");
                                u.fixture.startup_menu_stage += 1;
                                drop(b);
                                unsafe {
                                    let _: () = msg_send![self,toggleStartupApp:&*item];
                                }
                                return None;
                            }
                            return None;
                        }
                        if !s.workspace.startup_apps.is_empty() {
                            return None;
                        }
                        if !u.fixture.fixture_rename_tested {
                            if !u.fixture.fixture_rename_started {
                                u.fixture.fixture_rename_started = true;
                                u.client.send(Command::Raise);
                                let id = s.selected.unwrap();
                                drop(b);
                                self.begin_rename(id);
                                self.ivars()
                                    .ui
                                    .borrow()
                                    .as_ref()
                                    .unwrap()
                                    .client
                                    .send(Command::Raise);
                                return None;
                            }
                            if u.editing_focus_pending.is_some() {
                                return None;
                            }
                            // Exercise the production snapshot synchronization while
                            // a real native rename editor is open on its original tab.
                            let original = u.rename_editor.as_ref().unwrap().id;
                            let mut changed = s.clone();
                            changed.selected = s
                                .workspace
                                .tabs
                                .iter()
                                .map(|t| t.id)
                                .find(|id| *id != original);
                            sync_selection(u, &changed);
                            assert_eq!(u.rename_editor.as_ref().unwrap().id, original);
                            for view in u.tabs.subviews() {
                                if let Some(button) = view.downcast_ref::<TabButton>() {
                                    assert_eq!(
                                        button.ivars().active.get(),
                                        Some(button.tag() as TabId) == changed.selected
                                    );
                                }
                            }
                            sync_selection(u, s);
                            println!(
                                "Startup app menu toggle persisted and cleared the selected app rule"
                            );
                            println!(
                                "A2 snapshot update changed rendered selection and retained open rename target"
                            );
                            let field = u
                                .rename_editor
                                .as_ref()
                                .expect("Renaming ended while app was attached")
                                .field
                                .clone();
                            u.fixture.fixture_rename_tested = true;
                            u.fixture.dock_test_tick = count;
                            let manager = u.window.clone();
                            let number = s.backdrop.as_ref().unwrap().0;
                            let frame = s.selected_frame.unwrap();
                            drop(b);
                            self.verify_rename_keyboard(&field);
                            self.verify_app_pointer_routes(&manager, number, frame);
                            self.finish_rename(false);
                            println!(
                                "Attached-app rename retained keyboard focus despite queued Raise requests"
                            );
                            return None;
                        }
                        assert!(
                            s.workspace.geometry.height > 400.,
                            "Manager did not grow around native minimum size"
                        );
                    }
                    let wanted = s.workspace.tabs[(stage - 2) % 2].id;
                    u.client.switch(wanted);
                    println!("Integrated switch {} requested", stage - 1);
                    u.fixture.dock_test_stage += 1;
                    u.fixture.dock_test_tick = count;
                } else if stage == 14 && s.selected == Some(s.workspace.tabs[1].id) {
                    if u.fixture.fixture_child.is_some() && !u.fixture.fixture_reveal_tested {
                        if !u.fixture.fixture_reveal_started {
                            u.fixture.fixture_reveal_started = true;
                            u.fixture.dock_test_tick = count;
                            let window = u.window.clone();
                            let client = u.client.clone();
                            drop(b);
                            window.orderBack(None);
                            client.send(Command::Raise);
                            return None;
                        }
                        // The reveal itself verifies keyboard ownership immediately;
                        // desktop focus can legitimately change before this later stage.
                        u.fixture.dock_test_tick = count;
                        u.client.send(Command::Raise);
                        return None;
                    }
                    u.fixture.dock_test_stage = 15;
                    u.fixture.dock_test_tick = count;
                    let window = u.window.clone();
                    let mut f = window.frame();
                    f.origin.x += 35.;
                    f.size.width += 60.;
                    drop(b);
                    window.setFrame_display(f, true);
                    return None;
                } else if stage == 15 {
                    if std::env::args().any(|a| a == "--pointer-smoke") {
                        if !s.selected_frame.is_some_and(|frame| frame.near(s.area)) {
                            return None;
                        }
                        self.verify_app_pointer_routes(
                            &u.window,
                            s.backdrop.as_ref().unwrap().0,
                            s.selected_frame.unwrap(),
                        );
                    }
                    println!(
                        "Integrated docking and manager resize completed; releasing first tab"
                    );
                    if let Some(t) = s.workspace.tabs.first() {
                        u.client.send(Command::Release(t.id));
                    }
                    u.fixture.dock_test_stage = 16;
                    u.fixture.dock_test_tick = count;
                } else if stage == 16 && s.live.len() == 1 {
                    println!("Integrated release completed; restoring remaining window on exit");
                    u.client.send(Command::Quit);
                    u.fixture.dock_test_stage = 20;
                }
            }
        }
        Some(b)
    }
    pub(super) fn verify_app_pointer_routes(&self, window: &NSWindow, number: u32, frame: Rect) {
        let primary = NSScreen::screens(self.mtm())
            .firstObject()
            .unwrap()
            .frame()
            .size
            .height;
        for (x, y) in [(0.2, 0.3), (0.5, 0.5), (0.8, 0.8)] {
            let point = NSPoint::new(
                frame.x + frame.width * x,
                primary - frame.y - frame.height * y,
            );
            assert_eq!(
                NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(point, 0, self.mtm()),
                number as isize,
                "A window above the docked app is intercepting pointer input"
            );
        }
        let bounds = window.contentView().unwrap().bounds();
        let point = window
            .convertRectToScreen(rect(90., bounds.size.height - 16., 1., 1.))
            .origin;
        assert_eq!(
            NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(point, 0, self.mtm()),
            window.windowNumber(),
            "AppDock controls are not clickable"
        );
        println!(
            "Native pointer routing passed: app content targets the external app; tab controls target AppDock"
        );
    }
    pub(super) fn verify_rename_keyboard(&self, field: &NSTextField) {
        let editor = field
            .currentEditor()
            .expect("Rename field has no keyboard editor")
            .downcast::<RenameFieldEditor>()
            .expect("Wrong rename field editor");
        let app = NSApplication::sharedApplication(self.mtm());
        let foreground = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier());
        let child = self.ivars().ui.borrow().as_ref().and_then(|u| {
            u.fixture
                .fixture_child
                .as_ref()
                .map(|child| child.id() as i32)
        });
        println!(
            "Fixture keyboard context: active={}, key_window={}, foreground_manager={}, foreground_child={}",
            app.isActive(),
            field.window().is_some_and(|w| w.isKeyWindow()),
            foreground == Some(std::process::id() as i32),
            foreground == child
        );
        assert!(
            app.isActive(),
            "AppDock did not take keyboard focus for renaming"
        );
        let number = field.window().unwrap().windowNumber();
        let key = |modifiers, characters: &str| {
            NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                    NSEventType::KeyDown,NSPoint::new(0.,0.),modifiers,0.,number,None,
                    &NSString::from_str(characters),&NSString::from_str(characters),false,0,
                ).unwrap()
        };
        for modifiers in [NSEventModifierFlags::Control, NSEventModifierFlags::Command] {
            editor.setSelectedRange(objc2_foundation::NSRange::new(1, 0));
            app.sendEvent(&key(modifiers, "a"));
            assert_eq!(
                NSTextInputClient::selectedRange(&**editor),
                objc2_foundation::NSRange::new(0, editor.string().length()),
                "Select-all shortcut did not reach rename field"
            );
        }
        app.sendEvent(&key(NSEventModifierFlags::empty(), "x"));
        assert_eq!(
            editor.string().to_string(),
            "x",
            "Typed key did not replace rename selection"
        );
    }
}

#[path = "fixtures_review.rs"]
mod review;
pub(crate) use review::{run as review, targets as review_targets};
