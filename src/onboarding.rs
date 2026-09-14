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

pub fn installed(path: &std::path::Path, home: Option<&std::path::Path>) -> bool {
    running_app(path)
        .extension()
        .is_some_and(|ext| ext == "app")
        && (path.starts_with("/Applications")
            || home.is_some_and(|home| path.starts_with(home.join("Applications"))))
}
pub fn running_installed() -> bool {
    std::env::current_exe().is_ok_and(|path| {
        installed(
            &path,
            std::env::var_os("HOME")
                .as_deref()
                .map(std::path::Path::new),
        )
    })
}

/// Reveal the enclosing app bundle, or the executable for development launches.
pub fn running_app(path: &std::path::Path) -> &std::path::Path {
    path.ancestors()
        .find(|p| p.extension().is_some_and(|ext| ext == "app"))
        .unwrap_or(path)
}

pub fn picker_message(
    trusted: bool,
    complete: bool,
    failed: bool,
    eligible: usize,
    searching: bool,
) -> &'static str {
    if !trusted {
        "AppDock doesn’t have window access.\nOpen Setup & Diagnostics below for help enabling it.\nAlready enabled? macOS may remember a different copy."
    } else if failed {
        "Window access is granted, but the window check failed.\nOpen Setup & Diagnostics below, or try Refresh."
    } else if !complete {
        "Window access granted.\nChecking available windows…"
    } else if eligible == 0 {
        "Window access granted, but no compatible windows were found.\nOpen a normal Finder or TextEdit window, then click Refresh."
    } else if searching {
        "No matching windows.\nTry another search or clear the search field."
    } else {
        "No more windows are available to add.\nCompatible windows may already be in your workspace."
    }
}

/// Only allow signing metadata; never copy arbitrary tool output or window data.
pub fn signing_metadata(output: &str) -> String {
    let lines: Vec<_> = output
        .lines()
        .filter(|line| {
            [
                "Identifier=",
                "TeamIdentifier=",
                "Authority=",
                "CDHash=",
                "Signature=",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        })
        .collect();
    if lines.is_empty() {
        "Signing identity unavailable".into()
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_detection_covers_updates_and_rejects_downloads() {
        use std::path::Path;
        let home = Some(Path::new("/Users/example"));
        for path in [
            "/Applications/AppDock.app/Contents/MacOS/AppDock",
            "/Users/example/Applications/AppDock.app/Contents/MacOS/AppDock",
        ] {
            assert!(installed(Path::new(path), home));
        }
        for path in [
            "/Applications-copy/AppDock.app/Contents/MacOS/AppDock",
            "/Volumes/Install AppDock/AppDock.app/Contents/MacOS/AppDock",
            "/Users/example/Downloads/AppDock.app/Contents/MacOS/AppDock",
            "/tmp/target/debug/appdock",
        ] {
            assert!(!installed(Path::new(path), home));
        }
    }

    #[test]
    fn picker_explains_permission_before_search_or_old_results() {
        assert!(picker_message(false, true, false, 5, true).contains("doesn’t have window access"));
        assert!(picker_message(true, false, true, 0, false).contains("check failed"));
        assert!(picker_message(true, false, false, 0, false).contains("Checking"));
        assert!(picker_message(true, true, false, 0, false).contains("no compatible windows"));
        assert!(picker_message(true, true, false, 5, true).contains("No matching"));
        assert!(picker_message(true, true, false, 5, false).contains("already"));
    }
    #[test]
    fn diagnostics_exclude_unexpected_output_and_reveal_the_exact_bundle() {
        assert_eq!(
            signing_metadata(
                "Executable=/private/example\nIdentifier=dev.appdock.AppDock\nSecret window title\nTeamIdentifier=EXAMPLE"
            ),
            "Identifier=dev.appdock.AppDock\nTeamIdentifier=EXAMPLE"
        );
        assert_eq!(
            signing_metadata("tool failed"),
            "Signing identity unavailable"
        );
        use std::path::Path;
        assert_eq!(
            running_app(Path::new(
                "/Applications/AppDock.app/Contents/MacOS/AppDock"
            )),
            Path::new("/Applications/AppDock.app")
        );
        assert_eq!(
            running_app(Path::new("/tmp/target/debug/appdock")),
            Path::new("/tmp/target/debug/appdock")
        );
    }
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
