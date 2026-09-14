//! Sparkle is loaded only from the signed app's embedded Frameworks directory.
//! This keeps plain `cargo run` and deterministic tests independent of the SDK.
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{MainThreadMarker, NSBundle, NSString};

pub struct Updater {
    _framework: Retained<NSBundle>,
    controller: Retained<AnyObject>,
}
impl Updater {
    pub fn load(_main: MainThreadMarker, delegate: &AnyObject) -> Result<Self, String> {
        let app = NSBundle::mainBundle();
        let path = app.bundlePath().to_string();
        if !path.ends_with(".app") || crate::onboarding::from_installer(std::path::Path::new(&path))
        {
            return Err("Install AppDock in Applications before checking for updates.".into());
        }
        for key in ["SUFeedURL", "SUPublicEDKey"] {
            if app
                .objectForInfoDictionaryKey(&NSString::from_str(key))
                .is_none()
            {
                return Err("This development build does not include updates. Install a published AppDock release.".into());
            }
        }
        let framework = NSBundle::bundleWithPath(&NSString::from_str(&format!(
            "{path}/Contents/Frameworks/Sparkle.framework"
        )))
        .ok_or("The update component is missing. Reinstall AppDock from its release download.")?;
        // Loading is confined to a bundled framework; release library validation enforces its signer.
        unsafe { framework.loadAndReturnError() }.map_err(|e| e.to_string())?;
        let class = framework
            .classNamed(&NSString::from_str("SPUStandardUpdaterController"))
            .ok_or("The update component could not be initialized.")?;
        // These selectors and ownership conventions are from Sparkle's public Objective-C API.
        let controller: Retained<AnyObject> = unsafe {
            let allocated: *mut AnyObject = msg_send![class, alloc];
            let initialized: *mut AnyObject = msg_send![allocated, initWithStartingUpdater: false,
                updaterDelegate: delegate, userDriverDelegate: Option::<&AnyObject>::None];
            Retained::from_raw(initialized)
                .ok_or("Sparkle initialization returned no controller.")?
        };
        Ok(Self {
            _framework: framework,
            controller,
        })
    }
    pub fn start(&self) {
        unsafe {
            let _: () = msg_send![&*self.controller, startUpdater];
        }
    }
    pub fn check(&self) {
        unsafe {
            let _: () = msg_send![&*self.controller, checkForUpdates: Option::<&AnyObject>::None];
        }
    }
    pub fn can_check(&self) -> bool {
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.controller, updater];
            msg_send![&*updater, canCheckForUpdates]
        }
    }
    pub fn automatic_checks(&self) -> bool {
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.controller, updater];
            msg_send![&*updater, automaticallyChecksForUpdates]
        }
    }
    pub fn set_automatic_checks(&self, enabled: bool) {
        unsafe {
            let updater: Retained<AnyObject> = msg_send![&*self.controller, updater];
            let _: () = msg_send![&*updater, setAutomaticallyChecksForUpdates: enabled];
        }
    }
}
