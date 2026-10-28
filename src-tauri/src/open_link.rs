//! `shardbrowser://` links (fork).
//!
//! `open/<profile-id>` starts the profile exactly like the Open button does:
//! headed and without CDP. A profile that already runs here is raised instead
//! of started twice; one open on another team member's computer is refused by
//! the team lease like any other start.
//!
//! `create/<code>` makes a channel's profile in the team. The channel organizer
//! hands out the one-time code; the launcher redeems it there for the profile's
//! name and proxy (so no password travels in the link), makes proxy and profile
//! like `POST /profiles` does, and reports the new id back to link the channel.

use std::time::Duration;

const SCHEME: &str = "shardbrowser://";
/// The only organizer a code is redeemed at: a link from anywhere else names
/// no server, so it cannot point the launcher at one.
const ORGANIZER: &str = "https://channel-organizer.youtube-automation.de";

#[derive(Debug, PartialEq, Eq)]
enum Link {
    Open(String),
    Create(String),
}

/// Plain tokens only; anything else (paths, queries, escapes) is refused.
fn token(rest: &str, prefix: &str, len: std::ops::RangeInclusive<usize>) -> Option<String> {
    let value = rest.strip_prefix(prefix)?.trim_end_matches('/');
    let valid = len.contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| value.to_owned())
}

fn parse(url: &str) -> Option<Link> {
    let rest = url.trim().strip_prefix(SCHEME)?;
    token(rest, "open/", 1..=64)
        .map(Link::Open)
        .or_else(|| token(rest, "create/", 43..=43).map(Link::Create))
}

/// Whether a command line carries a link (a second instance started by Windows).
pub fn in_args(args: &[String]) -> bool {
    args.iter().any(|a| parse(a).is_some())
}

/// Acts on every link, in the background.
pub fn handle<S: AsRef<str>>(urls: impl IntoIterator<Item = S>, cold_start: bool) {
    for link in urls.into_iter().filter_map(|u| parse(u.as_ref())) {
        match link {
            Link::Open(id) => tauri::async_runtime::spawn(open(id, cold_start)),
            Link::Create(code) => tauri::async_runtime::spawn(create(code, cold_start)),
        };
    }
}

async fn settle(cold_start: bool) {
    if cold_start {
        // Started by the link itself: let the space and the team link come up first.
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn open(id: String, cold_start: bool) {
    settle(cold_start).await;
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

#[derive(serde::Deserialize)]
struct Provision {
    name: String,
    notes: String,
    proxy: Option<ProxyParts>,
}

#[derive(serde::Deserialize)]
struct ProxyParts {
    host: String,
    port: u16,
    username: String,
    password: String,
}

async fn create(code: String, cold_start: bool) {
    settle(cold_start).await;
    match make(&code).await {
        Ok(name) => crate::notify_warning(format!("Profil „{name}“ im Team angelegt und mit dem Kanal verknüpft.")),
        Err(e) => crate::notify_warning(format!("Profil konnte nicht angelegt werden: {e}")),
    }
}

async fn make(code: &str) -> Result<String, String> {
    let t = team_ticket().await?;
    let url = format!("{ORGANIZER}/api/browser/provision");
    let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())?;
    let asked = http.get(&url).bearer_auth(code).send().await.map_err(|e| format!("Channel-Organizer nicht erreichbar ({e})"))?;
    let p: Provision = answer(asked).await?.json().await.map_err(|e| e.to_string())?;
    let proxy_id = match p.proxy {
        Some(parts) => Some(add_proxy(&t, &p.name, parts).await?),
        None => None,
    };
    let id = crate::api::create_for_link(&t, p.name.clone(), p.notes, proxy_id).await?;
    crate::notify_store_changed("profiles");
    let told = http
        .post(&url)
        .bearer_auth(code)
        .json(&serde_json::json!({ "profileId": id }))
        .send()
        .await
        .map_err(|e| format!("„{}“ angelegt, aber der Channel-Organizer ist nicht erreichbar ({e})", p.name))?;
    answer(told).await.map_err(|e| format!("„{}“ angelegt, aber nicht verknüpft: {e}", p.name))?;
    Ok(p.name)
}

/// The organizer's error text for a refused request.
async fn answer(r: reqwest::Response) -> Result<reqwest::Response, String> {
    if r.status().is_success() {
        return Ok(r);
    }
    let status = r.status();
    let body: serde_json::Value = r.json().await.unwrap_or_default();
    Err(body.get("error").and_then(|v| v.as_str()).map(str::to_owned).unwrap_or_else(|| format!("HTTP {status}")))
}

/// Channel profiles belong to the team: the launcher moves into it first when
/// it works in the personal space.
async fn team_ticket() -> Result<crate::space::Ticket, String> {
    if crate::team::active_team().is_none() {
        let state = crate::team::state().map_err(|e| e.to_string())?;
        let team = state
            .teams
            .iter()
            .find(|t| !t.revoked && !t.needs_invite)
            .ok_or("ShardBrowser ist mit keinem Team verbunden.")?;
        let st = crate::space::switch(crate::team::Space::Team { team_id: team.id.clone() })
            .await
            .map_err(|e| e.to_string())?;
        crate::notify_event("space-changed", serde_json::to_value(&st.space).unwrap_or_default());
    }
    crate::space::ready().await;
    Ok(crate::space::Ticket::now())
}

/// The channel's proxy, kept in the team. The organizer stores no scheme:
/// providers serve HTTP, SOCKS5 or both on a port, so the one that answers is kept.
async fn add_proxy(t: &crate::space::Ticket, name: &str, p: ProxyParts) -> Result<String, String> {
    use crate::space::proxy::{full_test, probe, upsert_dedup, ProxyEntry, ProxyKind};
    let mut entry = ProxyEntry {
        id: String::new(),
        name: name.to_owned(),
        kind: ProxyKind::Http,
        host: p.host,
        port: p.port,
        username: p.username,
        password: p.password,
        country: String::new(),
        notes: "Channel-Organizer".into(),
    };
    if probe(&entry).await.is_err() {
        entry.kind = ProxyKind::Socks5;
        if probe(&entry).await.is_err() {
            entry.kind = ProxyKind::Http;
            crate::notify_warning(format!("Der Proxy für „{name}“ antwortet gerade nicht. Er wurde als HTTP angelegt – bitte in ShardBrowser prüfen."));
        }
    }
    let stored = upsert_dedup(crate::space::Door("proxy_save"), t, entry).await.map_err(|e| e.to_string())?;
    // Best effort, like the API: geo and UDP for the timezone and language a start resolves.
    let _ = full_test(&stored).await;
    crate::notify_store_changed("proxies");
    Ok(stored.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "Ab3_-defghijklmnopqrstuvwxyzABCDEFGHIJKLMN";

    #[test]
    fn reads_the_profile_from_an_open_link() {
        assert_eq!(parse("shardbrowser://open/k1i65jwh"), Some(Link::Open("k1i65jwh".into())));
        assert_eq!(parse("shardbrowser://open/3f2b-41_a/"), Some(Link::Open("3f2b-41_a".into())));
    }

    #[test]
    fn reads_the_code_from_a_create_link() {
        assert_eq!(CODE.len(), 43);
        assert_eq!(parse(&format!("shardbrowser://create/{CODE}")), Some(Link::Create(CODE.into())));
        assert_eq!(parse(&format!("shardbrowser://create/{CODE}x")), None);
        assert_eq!(parse("shardbrowser://create/short"), None);
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
            assert_eq!(parse(url), None, "{url}");
        }
        assert_eq!(parse(&format!("shardbrowser://open/{}", "a".repeat(65))), None);
    }

    #[test]
    fn spots_a_link_on_the_command_line() {
        assert!(in_args(&["launcher.exe".into(), "shardbrowser://open/abc".into()]));
        assert!(in_args(&["launcher.exe".into(), format!("shardbrowser://create/{CODE}")]));
        assert!(!in_args(&["launcher.exe".into()]));
    }
}
