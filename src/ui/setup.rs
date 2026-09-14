//! Setup lives inside the workspace. AX probes run on the existing worker.
use super::*;
use crate::onboarding::Step;

pub(super) struct SetupUi {
    view: Retained<NSScrollView>,
    document: Retained<NSView>,
    card: Retained<NSView>,
    install_graphic: Retained<NSView>,
    progress: Retained<NSTextField>,
    title: Retained<NSTextField>,
    detail: Retained<NSTextField>,
    result: Retained<NSTextField>,
    primary: Retained<NSButton>,
    secondary: Retained<NSButton>,
    back: Retained<NSButton>,
    step: Step,
    generation: u64,
    waiting: bool,
    finishing: bool,
    finish_generation: u64,
    test_started: bool,
}
impl SetupUi {
    fn new(delegate: &Delegate) -> Self {
        let m = delegate.mtm();
        let view = NSScrollView::initWithFrame(NSScrollView::alloc(m), rect(0., 0., 700., 560.));
        view.setHasVerticalScroller(true);
        view.setAutohidesScrollers(true);
        view.setDrawsBackground(true);
        view.setBackgroundColor(&theme::color(theme::SURFACE));
        view.setHidden(true);
        let document = NSView::initWithFrame(NSView::alloc(m), rect(0., 0., 700., 560.));
        let content = NSView::initWithFrame(NSView::alloc(m), rect(30., 20., 640., 520.));
        document.addSubview(&content);
        view.setDocumentView(Some(&document));
        let label = |text: &str, frame, size, color| {
            let field = NSTextField::wrappingLabelWithString(&NSString::from_str(text), m);
            field.setFrame(frame);
            field.setFont(Some(&theme::font(size)));
            field.setTextColor(Some(&theme::color(color)));
            content.addSubview(&field);
            field
        };
        let progress = label("", rect(32., 477., 576., 22.), 12., theme::SECONDARY);
        let title = label("", rect(32., 426., 576., 38.), 24., theme::TEXT);
        let detail = label("", rect(32., 286., 576., 132.), 14., theme::SECONDARY);
        let result = label("", rect(32., 90., 576., 118.), 13., theme::TEXT);
        let primary = delegate.button("Continue", sel!(setupNext:), rect(422., 28., 186., 32.));
        primary.setBordered(true);
        let secondary = delegate.button("", sel!(setupAction:), rect(158., 28., 254., 32.));
        let back = delegate.button("Back", sel!(setupBack:), rect(84., 28., 66., 32.));
        let later = delegate.button("Later", sel!(setupLater:), rect(22., 28., 60., 32.));
        for button in [&primary, &secondary, &back, &later] {
            content.addSubview(button);
        }
        let install_graphic = NSView::initWithFrame(NSView::alloc(m), rect(120., 180., 400., 112.));
        let app_icon = NSImage::initWithData(
            NSImage::alloc(),
            &NSData::with_bytes(include_bytes!("../../assets/branding/appdock.png")),
        )
        .unwrap();
        let folder_icon =
            NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str("/Applications"));
        for (x, icon, name) in [
            (16., app_icon, "AppDock"),
            (290., folder_icon, "Applications"),
        ] {
            let image = NSImageView::initWithFrame(NSImageView::alloc(m), rect(x, 30., 80., 80.));
            image.setImage(Some(&icon));
            image.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            install_graphic.addSubview(&image);
            let caption = NSTextField::labelWithString(&NSString::from_str(name), m);
            caption.setFrame(rect(x - 16., 4., 112., 22.));
            caption.setAlignment(NSTextAlignment::Center);
            caption.setTextColor(Some(&theme::color(theme::TEXT)));
            install_graphic.addSubview(&caption);
        }
        let arrow = NSTextField::labelWithString(&NSString::from_str("→"), m);
        arrow.setFont(Some(&theme::font(38.)));
        arrow.setTextColor(Some(&theme::color(theme::TAB_ACCENT)));
        arrow.setFrame(rect(172., 45., 70., 54.));
        install_graphic.addSubview(&arrow);
        content.addSubview(&install_graphic);
        Self {
            view,
            document,
            card: content,
            install_graphic,
            progress,
            title,
            detail,
            result,
            primary,
            secondary,
            back,
            step: Step::Install,
            generation: 0,
            waiting: false,
            finishing: false,
            finish_generation: 0,
            test_started: false,
        }
    }
    pub(super) fn layout(&self, bounds: NSRect) {
        let visible = self.view.contentView().bounds();
        let at_top =
            visible.origin.y + visible.size.height >= self.document.frame().size.height - 1.;
        self.view.setFrame(bounds);
        let width = bounds.size.width.max(640.);
        let height = bounds.size.height.max(520.);
        self.document.setFrameSize(NSSize::new(width, height));
        self.card
            .setFrameOrigin(NSPoint::new((width - 640.) / 2., (height - 520.) / 2.));
        if at_top {
            self.scroll_to_top();
        }
    }
    fn scroll_to_top(&self) {
        let clip = self.view.contentView();
        clip.scrollToPoint(NSPoint::new(
            0.,
            (self.document.frame().size.height - clip.bounds().size.height).max(0.),
        ));
        self.view.reflectScrolledClipView(&clip);
    }
    pub(super) fn visible(&self) -> bool {
        !self.view.isHidden()
    }
}

impl Delegate {
    pub(super) fn show_setup(&self) {
        self.dismiss_picker(false);
        self.finish_rename(true);
        let window = {
            let mut borrow = self.ivars().ui.borrow_mut();
            let Some(u) = borrow.as_mut() else { return };
            u.setup_auto_shown = true;
            u.pending_settings = false;
            u.client.set_text_editing(true);
            u.raise_after = None;
            if u.setup_wizard.is_none() {
                let setup = SetupUi::new(self);
                u.surface.addSubview(&setup.view);
                u.setup_wizard = Some(setup);
            }
            let setup = u.setup_wizard.as_mut().unwrap();
            if !setup.visible() && !setup.test_started && setup.finishing {
                setup.step = Step::Install;
                setup.finishing = false;
            }
            setup.layout(rect(
                FRAME,
                FRAME,
                u.surface.bounds().size.width - 2. * FRAME,
                (u.surface.bounds().size.height - u.surface.chrome() - 2. * FRAME).max(100.),
            ));
            setup.view.setHidden(false);
            setup.scroll_to_top();
            u.surface.set_attached(false);
            u.hint.setHidden(true);
            u.backdrop.hide();
            u.window.clone()
        };
        // No UI borrow across AppKit focus/visibility calls.
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        window.makeKeyAndOrderFront(None);
        self.refresh_setup();
    }
    pub(super) fn hide_setup(&self) {
        let mut borrow = self.ivars().ui.borrow_mut();
        let Some(u) = borrow.as_mut() else { return };
        if let Some(setup) = u.setup_wizard.as_mut() {
            setup.test_started = false;
            setup.view.setHidden(true);
        }
        u.client.set_text_editing(false);
        let s = u.client.snapshot.lock().unwrap();
        let attached = s
            .selected
            .is_some_and(|id| s.live.iter().any(|(tab, _)| *tab == id));
        u.surface.set_attached(attached && !u.picker_open);
        u.hint.setHidden(attached || u.picker_open);
    }
    pub(super) fn setup_back(&self) {
        if let Some(u) = self.ivars().ui.borrow_mut().as_mut()
            && let Some(setup) = u.setup_wizard.as_mut()
        {
            setup.step = setup.step.back();
            setup.scroll_to_top();
            setup.waiting = false;
            setup.finishing = false;
        }
        self.refresh_setup();
    }
    pub(super) fn setup_next(&self) {
        let installer_copy =
            std::env::current_exe().is_ok_and(|p| crate::onboarding::from_installer(&p));
        if installer_copy
            && self
                .ivars()
                .ui
                .borrow()
                .as_ref()
                .and_then(|u| u.setup_wizard.as_ref())
                .is_some_and(|s| s.step == Step::Install)
        {
            self.setup_action();
            self.close_request();
            return;
        }
        let mut choose = false;
        let mut verify = false;
        {
            let mut borrow = self.ivars().ui.borrow_mut();
            let Some(u) = borrow.as_mut() else { return };
            let s = u.client.snapshot.lock().unwrap().clone();
            let Some(setup) = u.setup_wizard.as_mut() else {
                return;
            };
            if setup.step == Step::TryWindow {
                choose = true;
            } else if setup.step == Step::Finish {
                if s.trusted && s.setup_docked && !s.paused {
                    setup.finishing = true;
                    setup.finish_generation += 1;
                    u.client
                        .send(Command::CompleteSetup(setup.finish_generation));
                }
            } else {
                let verified = !setup.waiting
                    && s.readiness.generation == setup.generation
                    && s.readiness.passed(s.trusted);
                setup.step = setup.step.next(s.trusted, verified, s.setup_docked);
                setup.scroll_to_top();
                verify = setup.step == Step::Verify;
            }
        }
        if choose {
            self.hide_setup();
            if let Some(u) = self.ivars().ui.borrow_mut().as_mut() {
                if let Some(setup) = u.setup_wizard.as_mut() {
                    setup.test_started = true;
                }
                u.client.send(Command::Resume);
            }
            self.show_picker(false);
        } else if verify {
            self.verify_setup();
        }
        self.refresh_setup();
    }
    pub(super) fn verify_setup(&self) {
        let mut borrow = self.ivars().ui.borrow_mut();
        let Some(u) = borrow.as_mut() else { return };
        let Some(setup) = u.setup_wizard.as_mut() else {
            return;
        };
        setup.generation += 1;
        setup.waiting = true;
        u.client.send(Command::VerifySetup(setup.generation));
    }
    pub(super) fn setup_action(&self) {
        let step = self
            .ivars()
            .ui
            .borrow()
            .as_ref()
            .and_then(|u| u.setup_wizard.as_ref())
            .map(|s| s.step);
        match step {
            Some(Step::Install) => {
                let url =
                    objc2_foundation::NSURL::fileURLWithPath(&NSString::from_str("/Applications"));
                NSWorkspace::sharedWorkspace().openURL(&url);
            }
            Some(Step::Permission) => self.open_accessibility(),
            Some(Step::Verify) => {
                self.verify_setup();
                self.refresh_setup();
            }
            _ => {}
        }
    }
    pub(super) fn open_accessibility(&self) {
        if let Some(u) = self.ivars().ui.borrow().as_ref() {
            u.client.send(Command::RequestPermission);
        }
        let url = objc2_foundation::NSURL::URLWithString(&NSString::from_str(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        ))
        .unwrap();
        NSWorkspace::sharedWorkspace().openURL(&url);
    }
    pub(super) fn refresh_setup(&self) {
        let mut show = false;
        let mut close = false;
        {
            let mut borrow = self.ivars().ui.borrow_mut();
            let Some(u) = borrow.as_mut() else { return };
            let s = u.client.snapshot.lock().unwrap().clone();
            let Some(setup) = u.setup_wizard.as_mut() else {
                return;
            };
            if !setup.visible() && !setup.test_started {
                return;
            }
            if setup.waiting && s.readiness.generation == setup.generation {
                setup.waiting = false;
            }
            if !s.trusted && setup.step.number() > 2 {
                setup.step = Step::Permission;
                setup.waiting = false;
                show = setup.test_started;
                setup.test_started = false;
            }
            if setup.test_started && s.setup_docked && !s.paused && s.selected.is_some() {
                setup.step = Step::TryWindow.next(s.trusted, true, true);
                setup.test_started = false;
                show = true;
            }
            if setup.finishing
                && let Some((generation, result)) = &s.setup_completion
                && *generation == setup.finish_generation
            {
                if result.is_ok() {
                    close = true;
                } else {
                    setup.finishing = false;
                }
            }
            setup.progress.setStringValue(&NSString::from_str(&format!(
                "STEP {} OF 5  ·  APPDOCK SETUP",
                setup.step.number()
            )));
            let path = std::env::current_exe()
                .ok()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Unavailable".into());
            let installer_copy = crate::onboarding::from_installer(std::path::Path::new(&path));
            let installed = path.starts_with("/Applications/")
                || std::env::var("HOME")
                    .is_ok_and(|home| path.starts_with(&format!("{home}/Applications/")));
            let (title, detail, result, primary, secondary, enabled) = match setup.step {
                Step::Install => (
                    "Install AppDock",
                    "Drag AppDock.app onto Applications in the installer window.\nThen eject the installer and open AppDock from Applications.\nAlready downloaded a ZIP? Move its AppDock.app into Applications first.",
                    format!(
                        "{}\nRunning copy (v{}):\n{path}",
                        if installed {
                            "Applications location verified."
                        } else if installer_copy {
                            "You opened the installer copy.\nInstall and reopen from Applications before granting access."
                        } else {
                            "This copy is outside Applications.\nInstalling there is recommended."
                        },
                        env!("CARGO_PKG_VERSION")
                    ),
                    if installer_copy {
                        "Quit & open Applications"
                    } else {
                        "Continue"
                    },
                    "Open Applications",
                    true,
                ),
                Step::Permission => (
                    "Allow window control",
                    "Open Accessibility settings and enable AppDock.\nThis lets it move, resize and focus the windows you choose.\nIf AppDock is already enabled but this check fails: remove its old entry, add this running copy with +, then quit and reopen AppDock.\nScreen Recording and Input Monitoring are not required.",
                    if s.trusted {
                        "Verified: macOS grants this running copy Accessibility access.".into()
                    } else {
                        "Not verified: macOS has not granted this process access.\nThis check updates automatically when you return.".into()
                    },
                    "Continue",
                    "Open Accessibility Settings",
                    s.trusted,
                ),
                Step::Verify => {
                    let result = if setup.waiting {
                        "Checking window access on the background worker…".into()
                    } else if let Some(error) = &s.readiness.error {
                        format!("Check failed: {error}")
                    } else if s.readiness.generation != setup.generation {
                        "Run the check to verify current access.".into()
                    } else if !s.readiness.desktop_available {
                        "Cannot read desktop window geometry.\nReopen AppDock in your logged-in desktop session and retry.".into()
                    } else if s.readiness.eligible_windows == 0 {
                        "No controllable windows found.\nOpen a normal Finder or TextEdit window on this desktop, leave fullscreen, close dialogs, then retry.".into()
                    } else {
                        format!(
                            "Verified: {} controllable window(s) and desktop geometry access.\n{}",
                            s.readiness.eligible_windows,
                            u.shortcut_error.as_deref().unwrap_or(
                                "Keyboard shortcuts are checked while AppDock is active."
                            )
                        )
                    };
                    (
                        "Check your workspace",
                        "AppDock checks real window access and the move, resize, minimize and raise capabilities needed for docking.\nNo window is changed during this check.\nSome apps and dialogs do not support these operations.",
                        result,
                        "Continue",
                        "Check again",
                        !setup.waiting
                            && s.readiness.generation == setup.generation
                            && s.readiness.passed(s.trusted),
                    )
                }
                Step::TryWindow => (
                    "Try your first window",
                    "Choose a window in Add App.\nAppDock will dock it and verify its position and focus.\nThe window remains in your workspace.\nOnce docked, click inside the app to check interaction.\nClose its AppDock tab whenever you want to release it; the app keeps running.",
                    if setup.test_started {
                        format!(
                            "Waiting for a successful dock.\n{}\nUse AppDock → Setup & Diagnostics to return here.\nIf an attempt left a failed tab, release it before retrying.",
                            s.status
                        )
                    } else {
                        "Choose an ordinary app window that can move and resize.".into()
                    },
                    "Choose a window",
                    "",
                    s.trusted,
                ),
                Step::Finish => (
                    "Window control verified",
                    "AppDock successfully docked your window and checked its frame, focus and desktop window identity.\nConfirm that you can click and type in the docked app, then finish setup.\nYou can rerun these checks from AppDock → Setup & Diagnostics.",
                    if setup.finishing {
                        format!("Saving setup… {}", s.status)
                    } else if let Some((_, Err(error))) = &s.setup_completion {
                        format!("Could not save setup: {error}.\nTry Finish setup again.")
                    } else {
                        u.shortcut_error.clone().unwrap_or_else(|| {
                            "Accessibility and a real docking operation passed.".into()
                        })
                    },
                    "Confirm & finish",
                    "",
                    s.trusted && s.setup_docked && !s.paused && !setup.finishing,
                ),
            };
            setup.install_graphic.setHidden(setup.step != Step::Install);
            setup.result.setFrame(if setup.step == Step::Install {
                rect(32., 78., 576., 92.)
            } else {
                rect(32., 130., 576., 130.)
            });
            setup.title.setStringValue(&NSString::from_str(title));
            setup.detail.setStringValue(&NSString::from_str(detail));
            setup.result.setStringValue(&NSString::from_str(&result));
            setup.primary.setTitle(&NSString::from_str(primary));
            setup.primary.setEnabled(enabled);
            setup.secondary.setTitle(&NSString::from_str(secondary));
            setup.secondary.setHidden(secondary.is_empty());
            setup.secondary.setEnabled(!setup.waiting);
            setup
                .back
                .setEnabled(setup.step != Step::Install && !setup.finishing);
        }
        if close {
            self.hide_setup();
        } else if show {
            self.show_setup();
        }
    }
}
