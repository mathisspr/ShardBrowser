//! `shardbrowser://open/<profile-id>` links. A click on such a link (from the
//! channel organizer, a bookmark, anywhere) starts the profile exactly like the
//! Open button does: headed and without CDP. A profile that already runs here
//! is raised instead of started twice; one open on another team member's
//! computer is refused by the team lease like any other start.

use std::time::Duration;

const SCHEME: &str = "shardbrowser://";

/// The profile a link names, or None for anything that is not an open link.
/// Ids are plain tokens; anything else (paths, queries, escapes) is refused.
pub fn profile_id(url: &str) -> Option<String> {
    let rest = url.trim().strip_prefix(SCHEME)?;
    let id = rest.strip_prefix("open/")?.trim_end_matches('/');
    let valid = !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| id.to_owned())
}

/// Whether a command line carries an open link (a second instance started by Windows).
pub fn in_args(args: &[String]) -> bool {
    args.iter().any(|a| profile_id(a).is_some())
}

/// Opens every profile the links name, in the background.
pub fn handle<S: AsRef<str>>(urls: impl IntoIterator<Item = S>, cold_start: bool) {
    for id in urls.into_iter().filter_map(|u| profile_id(u.as_ref())) {
        tauri::async_runtime::spawn(open(id, cold_start));
    }
}

async fn open(id: String, cold_start: bool) {
    if cold_start {
        // Started by the link itself: let the space and the team link come up first.
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    if crate::process::Tracker::shared().is_running(&id) {
        if let Ok(bus) = crate::bus().await {
            if bus.focus(&id) {
                return;
            }
        }
        crate::notify_warning("Dieses Profil ist bereits geöffnet.");
        return;
    }
    if crate::migrate::in_progress() {
        crate::notify_warning("Profile werden gerade verschoben – bitte gleich noch einmal versuchen.");
        return;
    }
    let bus = match crate::bus().await {
        Ok(bus) => bus,
        Err(e) => return crate::notify_warning(format!("Profil konnte nicht geöffnet werden: {e}")),
    };
    // Same call as the UI's `launch` command: no CDP, headed.
    if let Err(e) = crate::launch::launch_profile_synced(&id, false, false, None, bus.port, &bus.token).await {
        crate::notify_warning(format!("Profil konnte nicht geöffnet werden: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_profile_from_an_open_link() {
        assert_eq!(profile_id("shardbrowser://open/k1i65jwh"), Some("k1i65jwh".into()));
        assert_eq!(profile_id("shardbrowser://open/3f2b-41_a/"), Some("3f2b-41_a".into()));
    }

    #[test]
    fn refuses_anything_else() {
        for url in [
            "shardbrowser://open/",
            "shardbrowser://open/../etc",
            "shardbrowser://open/a?b=c",
            "shardbrowser://open/a%20b",
            "shardbrowser://delete/abc",
            "https://example.com/open/abc",
        ] {
            assert_eq!(profile_id(url), None, "{url}");
        }
        assert_eq!(profile_id(&format!("shardbrowser://open/{}", "a".repeat(65))), None);
    }

    #[test]
    fn spots_a_link_on_the_command_line() {
        assert!(in_args(&["launcher.exe".into(), "shardbrowser://open/abc".into()]));
        assert!(!in_args(&["launcher.exe".into()]));
    }
}
