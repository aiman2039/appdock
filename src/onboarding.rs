//! Setup evidence is session-local. A saved completion flag never grants access.
#[derive(Clone, Debug, Default)]
pub struct Readiness {
    pub generation: u64,
    pub eligible_windows: usize,
    pub desktop_available: bool,
    pub error: Option<String>,
}
impl Readiness {
    pub fn passed(&self, trusted: bool) -> bool {
        trusted
            && self.generation > 0
            && self.eligible_windows > 0
            && self.desktop_available
            && self.error.is_none()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
    Install,
    Permission,
    Verify,
    TryWindow,
    Finish,
}
impl Step {
    pub fn number(self) -> usize {
        match self {
            Self::Install => 1,
            Self::Permission => 2,
            Self::Verify => 3,
            Self::TryWindow => 4,
            Self::Finish => 5,
        }
    }
    pub fn next(self, trusted: bool, verified: bool, docked: bool) -> Self {
        match self {
            Self::Install => Self::Permission,
            Self::Permission if trusted => Self::Verify,
            Self::Verify if verified => Self::TryWindow,
            Self::TryWindow if trusted && docked => Self::Finish,
            _ => self,
        }
    }
    pub fn back(self) -> Self {
        match self {
            Self::Install | Self::Permission => Self::Install,
            Self::Verify => Self::Permission,
            Self::TryWindow => Self::Verify,
            Self::Finish => Self::TryWindow,
        }
    }
}

/// Installer and translocated copies must not become the permission identity.
pub fn from_installer(path: &std::path::Path) -> bool {
    path.starts_with("/Volumes")
        || path
            .components()
            .any(|part| part.as_os_str() == "AppTranslocation")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installer_detection_distinguishes_installed_and_development_copies() {
        use std::path::Path;
        assert!(from_installer(Path::new(
            "/Volumes/AppDock/AppDock.app/Contents/MacOS/AppDock"
        )));
        assert!(from_installer(Path::new(
            "/private/var/folders/example/AppTranslocation/id/d/AppDock.app/Contents/MacOS/AppDock"
        )));
        assert!(!from_installer(Path::new(
            "/Applications/AppDock.app/Contents/MacOS/AppDock"
        )));
        assert!(!from_installer(Path::new(
            "/Users/example/project/target/debug/appdock"
        )));
    }
    #[test]
    fn permission_and_live_evidence_gate_progress() {
        assert_eq!(Step::Permission.next(false, true, true), Step::Permission);
        assert_eq!(Step::Verify.next(true, false, true), Step::Verify);
        assert_eq!(Step::TryWindow.next(true, true, false), Step::TryWindow);
        assert_eq!(Step::TryWindow.next(false, true, true), Step::TryWindow);
        assert_eq!(Step::TryWindow.next(true, true, true), Step::Finish);
    }
    #[test]
    fn empty_desktop_errors_and_revoked_access_are_not_ready() {
        let mut result = Readiness::default();
        assert!(!result.passed(true));
        result.generation = 1;
        result.eligible_windows = 1;
        assert!(!result.passed(true));
        result.desktop_available = true;
        assert!(result.passed(true));
        assert!(!result.passed(false));
        result.error = Some("Timed out".into());
        assert!(!result.passed(true));
    }
}
