//! Native tab-overflow reproduction and regression. Uses dummy tabs only.
use super::*;

#[derive(Default)]
pub(super) struct State {
    frame: Option<NSRect>,
    area: Option<Rect>,
    browsed_offset: f64,
}

impl Delegate {
    pub(super) fn tabs_fixture_step(&self, count: u64) {
        let mut b = self.ivars().ui.borrow_mut();
        let u = b.as_mut().unwrap();
        let clip = u.tab_scroll.contentView().bounds();
        match count {
            6 => {
                // Recreate the old fixed-height strip with a legacy scroller.
                // The style is local to this disposable view, not a system setting.
                let legacy = NSScrollView::initWithFrame(
                    NSScrollView::alloc(self.mtm()),
                    rect(0., 0., 600., 34.),
                );
                legacy.setHasHorizontalScroller(true);
                legacy.setAutohidesScrollers(true);
                legacy.setScrollerStyle(NSScrollerStyle::Legacy);
                let document =
                    NSView::initWithFrame(NSView::alloc(self.mtm()), rect(0., 0., 480., 32.));
                legacy.setDocumentView(Some(&document));
                u.surface.addSubview(&legacy);
                legacy.tile();
                let before = legacy.contentSize().height;
                document.setFrameSize(NSSize::new(1920., 32.));
                legacy.tile();
                legacy.reflectScrolledClipView(&legacy.contentView());
                let after = legacy.contentSize().height;
                println!(
                    "Overflow height reproduction: old viewport before={before}, after={after}, tab height=32"
                );
                assert!(
                    before >= 32. && after < 32.,
                    "Old scroller clipping was not reproduced"
                );
                legacy.removeFromSuperview();
                assert_eq!(clip.size.height, 32.);
                assert!(u.tab_scroll.arrows().0.isHidden());
                u.tabs_fixture.frame = Some(u.window.frame());
                u.tabs_fixture.area = u.last_area;
            }
            10 => {
                assert_eq!(
                    u.window.frame(),
                    u.tabs_fixture.frame.unwrap(),
                    "Adding tabs changed the workspace height/frame"
                );
                assert_eq!(
                    u.last_area, u.tabs_fixture.area,
                    "Adding tabs changed the app area"
                );
                assert_eq!(clip.size.height, 32., "Overflow clipped tab height");
                assert_eq!(clip.origin.y, 0.);
                assert!(
                    !u.tab_scroll.hasHorizontalScroller() && !u.tab_scroll.hasVerticalScroller()
                );
                let (left, right) = u.tab_scroll.arrows();
                assert!(!left.isHidden() && !right.isHidden());
                assert!(!left.isEnabled() && right.isEnabled());
                unsafe {
                    right.performClick(None);
                }
                assert!(u.tab_scroll.contentView().bounds().origin.x > 0.);
                for _ in 0..20 {
                    unsafe {
                        right.performClick(None);
                    }
                }
                assert!(!right.isEnabled() && left.isEnabled());
                for _ in 0..20 {
                    unsafe {
                        left.performClick(None);
                    }
                }
                assert_eq!(u.tab_scroll.contentView().bounds().origin.x, 0.);
                let event = objc2_core_graphics::CGEvent::new_scroll_wheel_event2(
                    None,
                    objc2_core_graphics::CGScrollEventUnit::Pixel,
                    2,
                    0,
                    -30,
                    0,
                )
                .unwrap();
                u.tab_scroll
                    .scrollWheel(&NSEvent::eventWithCGEvent(&event).unwrap());
                let horizontal = u.tab_scroll.contentView().bounds().origin.x;
                assert!(horizontal > 0., "Horizontal trackpad delta did not scroll");
                let event = objc2_core_graphics::CGEvent::new_scroll_wheel_event2(
                    None,
                    objc2_core_graphics::CGScrollEventUnit::Line,
                    1,
                    -1,
                    0,
                    0,
                )
                .unwrap();
                u.tab_scroll
                    .scrollWheel(&NSEvent::eventWithCGEvent(&event).unwrap());
                assert!(
                    u.tab_scroll.contentView().bounds().origin.x > horizontal,
                    "Mouse wheel did not scroll tabs"
                );
                assert_eq!(u.tab_scroll.contentView().bounds().origin.y, 0.);
                let s = u.client.snapshot.lock().unwrap();
                u.editing = s.workspace.tabs.last().map(|t| t.id);
                println!(
                    "Overflow: arrows, end bounds, trackpad and mouse-wheel scrolling passed without a scrollbar or geometry change"
                );
            }
            12 => {
                assert!(
                    clip.origin.x + clip.size.width >= 12. * TAB_WIDTH - 1.,
                    "Newly selected last tab stayed offscreen"
                );
                save_view(&u.surface);
                u.tab_scroll.scroll_by(-TAB_WIDTH);
                u.tabs_fixture.browsed_offset = u.tab_scroll.contentView().bounds().origin.x;
            }
            14 => {
                assert_eq!(
                    clip.origin.x, u.tabs_fixture.browsed_offset,
                    "Refresh snapped back after manual browsing"
                );
                let id = u
                    .client
                    .snapshot
                    .lock()
                    .unwrap()
                    .workspace
                    .tabs
                    .last()
                    .unwrap()
                    .id;
                drop(b);
                self.begin_rename(id);
            }
            22 => {
                let editor = u
                    .rename_editor
                    .as_ref()
                    .expect("Offscreen tab rename did not open");
                assert!(editor.field.frame().origin.x >= clip.origin.x);
                assert!(
                    editor.field.frame().origin.x + editor.field.frame().size.width
                        <= clip.origin.x + clip.size.width
                );
                assert_eq!(clip.origin.y, 0., "Rename shifted the tab row vertically");
                assert_eq!(clip.size.height, 32.);
                drop(b);
                self.finish_rename(false);
            }
            26 => {
                assert!(u.tab_scroll.arrows().0.isHidden() && u.tab_scroll.arrows().1.isHidden());
                assert_eq!(clip.origin, NSPoint::new(0., 0.));
                assert_eq!(clip.size.height, 32.);
                assert_eq!(u.window.frame(), u.tabs_fixture.frame.unwrap());
                assert_eq!(u.last_area, u.tabs_fixture.area);
                let window = u.window.clone();
                let mut frame = window.frame();
                frame.size.width += 100.;
                drop(b);
                window.setFrame_display(frame, true);
            }
            30 => {
                assert_eq!(clip.size.height, 32.);
                assert_eq!(
                    u.window.frame().size.height,
                    u.tabs_fixture.frame.unwrap().size.height
                );
                assert!(u.tab_scroll.arrows().0.isHidden() && u.tab_scroll.arrows().1.isHidden());
                println!(
                    "Overflow: selection reveal, manual scroll retention, full-height rename, tab removal and window resize passed"
                );
                u.client.send(Command::Quit);
            }
            _ => {}
        }
    }
}
fn save_view(view: &NSView) {
    let bounds = view.bounds();
    let bitmap = view.bitmapImageRepForCachingDisplayInRect(bounds).unwrap();
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
    let data = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .unwrap();
    let output = std::env::var("APPDOCK_SMOKE_SCREENSHOT").unwrap();
    assert!(data.writeToFile_atomically(&NSString::from_str(&output), true));
}
