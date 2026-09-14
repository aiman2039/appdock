//! Horizontal tab navigation with a fixed viewport height and no native scrollers.
use super::*;

const ROW_HEIGHT: f64 = 32.;
const ARROW_SPACE: f64 = 30.;

#[derive(Default)]
pub(super) struct TabScrollIvars {
    arrows: OnceCell<(Retained<NSButton>, Retained<NSButton>)>,
}
define_class!(
    #[unsafe(super=NSScrollView)]
    #[thread_kind=MainThreadOnly]
    #[ivars=TabScrollIvars]
    pub(super) struct TabScrollView;
    unsafe impl NSObjectProtocol for TabScrollView {}
    impl TabScrollView {
        #[unsafe(method(scrollTabsLeft:))]
        fn left_action(&self, _: &AnyObject) { self.scroll_by(-TAB_WIDTH); }
        #[unsafe(method(scrollTabsRight:))]
        fn right_action(&self, _: &AnyObject) { self.scroll_by(TAB_WIDTH); }
        #[unsafe(method(scrollWheel:))]
        fn wheel(&self, event: &NSEvent) {
            // AppKit already applies the user's natural-scrolling preference.
            // Accept horizontal trackpad gestures and vertical mouse wheels here;
            // this view only receives events from the tab strip, not app content.
            let x = event.scrollingDeltaX();
            let y = event.scrollingDeltaY();
            let delta = if x.abs() >= y.abs() { x } else { y };
            let scale = if event.hasPreciseScrollingDeltas() { 1. } else { 32. };
            self.scroll_by(-delta * scale);
        }
    }
);
impl TabScrollView {
    pub(super) fn new(m: MainThreadMarker, parent: &NSView) -> Retained<Self> {
        let scroll: Retained<Self> = unsafe {
            msg_send![super(Self::alloc(m).set_ivars(TabScrollIvars::default())), initWithFrame: rect(0., 0., 100., ROW_HEIGHT)]
        };
        scroll.setHasHorizontalScroller(false);
        scroll.setHasVerticalScroller(false);
        scroll.setBorderType(NSBorderType::NoBorder);
        scroll.setDrawsBackground(false);
        scroll.setAutomaticallyAdjustsContentInsets(false);
        scroll.setHorizontalScrollElasticity(NSScrollElasticity::None);
        scroll.setVerticalScrollElasticity(NSScrollElasticity::None);
        scroll.setClipsToBounds(true);
        scroll.contentView().setClipsToBounds(true);
        let button = |text: &str, action, description: &str| {
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(text),
                    Some(&scroll),
                    Some(action),
                    m,
                )
            };
            button.setBordered(false);
            button.setFont(Some(&theme::font(20.)));
            button.setContentTintColor(Some(&theme::color(theme::TEXT)));
            button.setToolTip(Some(&NSString::from_str(description)));
            button.setAccessibilityLabel(Some(&NSString::from_str(description)));
            button.setHidden(true);
            parent.addSubview(&button);
            button
        };
        let left = button("‹", sel!(scrollTabsLeft:), "Scroll tabs left");
        let right = button("›", sel!(scrollTabsRight:), "Scroll tabs right");
        scroll.ivars().arrows.set((left, right)).unwrap();
        parent.addSubview(&scroll);
        scroll
    }
    pub(super) fn arrows(&self) -> (&NSButton, &NSButton) {
        let (left, right) = self.ivars().arrows.get().unwrap();
        (left, right)
    }
    pub(super) fn layout(&self, bounds: NSRect) {
        let width = (bounds.size.width - 2. * FRAME).max(1.);
        let document_width = self
            .documentView()
            .map_or(0., |view| view.frame().size.width);
        let overflow = document_width > width;
        let inset = if overflow { ARROW_SPACE } else { 0. };
        // Both the viewport and the document stay 32 points tall. Adding tabs
        // never inserts a scroller or consumes vertical space from this row.
        let y = bounds.size.height - CHROME + 2.;
        self.setFrame(rect(
            FRAME + inset,
            y,
            (width - 2. * inset).max(1.),
            ROW_HEIGHT,
        ));
        let (left, right) = self.arrows();
        left.setFrame(rect(FRAME, y, ARROW_SPACE - 2., ROW_HEIGHT));
        right.setFrame(rect(
            bounds.size.width - FRAME - ARROW_SPACE + 2.,
            y,
            ARROW_SPACE - 2.,
            ROW_HEIGHT,
        ));
        left.setHidden(!overflow);
        right.setHidden(!overflow);
        self.scroll_to(self.contentView().bounds().origin.x);
    }
    fn max_offset(&self) -> f64 {
        (self
            .documentView()
            .map_or(0., |view| view.frame().size.width)
            - self.contentSize().width)
            .max(0.)
    }
    fn scroll_to(&self, offset: f64) {
        let max = self.max_offset();
        let offset = offset.clamp(0., max);
        let clip = self.contentView();
        clip.scrollToPoint(NSPoint::new(offset, 0.));
        self.reflectScrolledClipView(&clip);
        let (left, right) = self.arrows();
        left.setEnabled(offset > 0.5);
        right.setEnabled(offset < max - 0.5);
    }
    pub(super) fn scroll_by(&self, delta: f64) {
        if delta.is_finite() {
            self.scroll_to(self.contentView().bounds().origin.x + delta);
        }
    }
    pub(super) fn reveal_tab(&self, index: usize) {
        let left = index as f64 * TAB_WIDTH;
        let visible = self.contentView().bounds();
        let offset = if left < visible.origin.x {
            left
        } else if left + TAB_WIDTH > visible.origin.x + visible.size.width {
            left + TAB_WIDTH - visible.size.width
        } else {
            visible.origin.x
        };
        self.scroll_to(offset);
    }
}
