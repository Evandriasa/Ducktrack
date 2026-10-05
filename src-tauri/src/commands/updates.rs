use std::process::Command;

use serde::Serialize;
use tauri::AppHandle;

use super::super::error::AppError;

pub const REPO_OWNER: &str = "Evandriasa";
pub const REPO_NAME: &str = "Ducktrack";
pub const REPO_URL: &str = "https://github.com/Evandriasa/Ducktrack";
const RELEASES_URL: &str = "https://github.com/Evandriasa/Ducktrack/releases";
const API_LATEST: &str = "https://api.github.com/repos/Evandriasa/Ducktrack/releases/latest";
const API_LIST: &str = "https://api.github.com/repos/Evandriasa/Ducktrack/releases?per_page=5";

const USER_AGENT: &str = concat!("DuckTrack/", env!("CARGO_PKG_VERSION"));

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub update_available: bool,
    pub repo_url: String,
    pub releases_url: String,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub notes: Option<String>,
    pub release_found: bool,
    pub message: String,
}

/// Extract the numeric core of a version tag, e.g. "v1.2.3", "release-2.0" or
/// "ducktrack-v3.0.0-beta.4" all yield a comparable numeric vector.
///
/// Any leading text is skipped, and pre-release / build metadata (`-beta.4`,
/// `+build`) is ignored so that "1.2.3-beta.1" ranks equal to "1.2.3" and users
/// are never pushed onto a pre-release.
fn parse_version(v: &str) -> Vec<u64> {
    let core = v.trim();
    let Some(first_digit) = core.find(|c: char| c.is_ascii_digit()) else {
        return Vec::new();
    };
    core[first_digit..]
        .split(['-', '+', ' '])
        .next()
        .unwrap_or("")
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|c: &char| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let l = parse_version(latest);
    let c = parse_version(current);
    for i in 0..l.len().max(c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv != cv {
            return lv > cv;
        }
    }
    false
}

fn http_get_json(url: &str) -> Result<(u16, serde_json::Value), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(12)))
        .https_only(true)
        // Inspect 4xx ourselves so a missing release falls through to the list endpoint.
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    Ok((status, value))
}

fn fetch_release() -> Result<Option<serde_json::Value>, String> {
    match http_get_json(API_LATEST) {
        Ok((200, value)) => Ok(Some(value)),
        Ok((404, _)) => {
            // No "latest" release yet: fall back to the newest published one.
            let (list_status, value) = http_get_json(API_LIST)?;
            if list_status == 404 {
                // Repository not visible (private or not created yet).
                return Ok(None);
            }
            if list_status != 200 {
                return Err(format!("GitHub returned HTTP {list_status}."));
            }
            let items = value.as_array().cloned().unwrap_or_default();
            // Prefer a real published release; never surface a pre-release or a
            // draft as an installable update.
            let is_draft = |r: &&serde_json::Value| {
                r.get("draft").and_then(|d| d.as_bool()) == Some(true)
            };
            let is_pre = |r: &&serde_json::Value| {
                r.get("prerelease").and_then(|d| d.as_bool()) == Some(true)
            };
            let published = items
                .iter()
                .find(|r| !is_draft(r) && !is_pre(r))
                .cloned()
                .or_else(|| items.iter().find(|r| !is_draft(r)).cloned())
                .or_else(|| items.first().cloned());
            Ok(published)
        }
        Ok((status, _)) => Err(format!("GitHub returned HTTP {status}.")),
        Err(e) => Err(e),
    }
}

fn summarize(json: &serde_json::Value) -> Option<String> {
    json.get("body")
        .and_then(|b| b.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Keep the banner short; the full notes live on the release page.
            if s.chars().count() > 400 {
                let cut: String = s.chars().take(400).collect();
                format!("{cut}…")
            } else {
                s
            }
        })
}

/// Turn a fetched release (or a fetch failure) into user-facing update info.
///
/// Kept free of Tauri types so the decision logic can be tested offline.
fn build_info(
    current: String,
    fetched: Result<Option<serde_json::Value>, String>,
) -> UpdateInfo {
    let base = UpdateInfo {
        current: current.clone(),
        latest: None,
        update_available: false,
        repo_url: REPO_URL.to_string(),
        releases_url: RELEASES_URL.to_string(),
        release_url: None,
        published_at: None,
        notes: None,
        release_found: false,
        message: String::new(),
    };

    let release = match fetched {
        Ok(Some(r)) => r,
        Ok(None) => {
            return UpdateInfo {
                message: "No published releases found on GitHub yet.".to_string(),
                ..base
            }
        }
        Err(e) => {
            return UpdateInfo {
                message: format!("Could not reach GitHub: {e}"),
                ..base
            }
        }
    };

    let latest = release
        .get("tag_name")
        .and_then(|t| t.as_str())
        .map(|s| s.trim_start_matches(['v', 'V']).to_string());
    let update_available = latest
        .as_ref()
        .map(|l| is_newer(l, &current))
        .unwrap_or(false);
    let message = match latest.as_ref() {
        Some(l) if update_available => format!("Version {l} is available — you have {current}."),
        Some(_) => format!("DuckTrack {current} is up to date."),
        None => "No published releases found on GitHub yet.".to_string(),
    };

    UpdateInfo {
        latest,
        update_available,
        release_found: true,
        message,
        release_url: release
            .get("html_url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string()),
        published_at: release
            .get("published_at")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string()),
        notes: summarize(&release),
        ..base
    }
}

/// Compare the running version against the newest GitHub release.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, AppError> {
    let current = app.package_info().version.to_string();

    let fetched = tauri::async_runtime::spawn_blocking(move || fetch_release())
        .await
        .map_err(|e| AppError::Other(format!("Update check failed: {e}")))?;

    Ok(build_info(current, fetched))
}

/// Open a DuckTrack GitHub page in the system browser.
#[tauri::command]
pub fn open_github_page(page: Option<String>) -> Result<(), AppError> {
    let url = match page.as_deref() {
        None | Some("") | Some("repo") => REPO_URL.to_string(),
        Some("releases") => RELEASES_URL.to_string(),
        Some(_) => {
            return Err(AppError::Validation(
                "Only the repository or releases page can be opened.".into(),
            ))
        }
    };
    open_url_in_browser(&url)
}

fn open_url_in_browser(url: &str) -> Result<(), AppError> {
    // Only ever open trusted GitHub URLs through the OS handler.
    if !(url.starts_with("https://github.com/") || url == REPO_URL) {
        return Err(AppError::Validation("Refusing to open untrusted URL.".into()));
    }

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("start").arg("").arg(url);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };

    cmd.spawn()
        .map_err(|e| AppError::Other(format!("Could not open browser: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("v1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("0.1.0"), vec![0, 1, 0]);
        assert_eq!(parse_version("V2.0"), vec![2, 0]);
        // Pre-release / build metadata is not part of the comparable core.
        assert_eq!(parse_version("1.2.3-beta.1"), vec![1, 2, 3]);
        assert_eq!(parse_version("1.2.3+build.7"), vec![1, 2, 3]);
        // Real-world GitHub tags seen in the wild: a text prefix must not eat
        // the major version number.
        assert_eq!(parse_version("tauri-v3.0.0-alpha.4"), vec![3, 0, 0]);
        assert_eq!(parse_version("ducktrack-v0.2.0"), vec![0, 2, 0]);
        assert_eq!(parse_version("release-2.0"), vec![2, 0]);
        // No digits at all must not panic.
        assert!(parse_version("nightly").is_empty());
    }

    #[test]
    fn detects_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("v0.1", "0.1.0"));
        // Numeric, not lexical, ordering.
        assert!(is_newer("0.10.0", "0.9.0"));
        // Prefixed tags must still compare correctly.
        assert!(is_newer("tauri-v3.0.0-alpha.4", "0.1.0"));
        assert!(!is_newer("ducktrack-v0.1.0", "0.1.0"));
        // A pre-release is never an upgrade over the same base version.
        assert!(!is_newer("1.2.3-beta.1", "1.2.3"));
    }

    fn release_json(tag: &str, body: &str) -> serde_json::Value {
        serde_json::json!({
            "tag_name": tag,
            "html_url": format!("https://github.com/{REPO_OWNER}/{REPO_NAME}/releases/tag/{tag}"),
            "published_at": "2026-01-15T10:00:00Z",
            "draft": false,
            "prerelease": false,
            "body": body,
        })
    }

    #[test]
    fn reports_update_available() {
        let info = build_info(
            "0.1.0".to_string(),
            Ok(Some(release_json("v0.2.0", "Adds the update checker."))),
        );
        assert!(info.release_found);
        assert!(info.update_available);
        assert_eq!(info.latest.as_deref(), Some("0.2.0"));
        assert_eq!(info.message, "Version 0.2.0 is available — you have 0.1.0.");
        assert_eq!(info.notes.as_deref(), Some("Adds the update checker."));
        assert!(info
            .release_url
            .as_deref()
            .unwrap()
            .ends_with("/releases/tag/v0.2.0"));
        assert_eq!(info.published_at.as_deref(), Some("2026-01-15T10:00:00Z"));
    }

    #[test]
    fn reports_up_to_date_when_tags_match() {
        let info = build_info("0.2.0".to_string(), Ok(Some(release_json("v0.2.0", "notes"))));
        assert!(info.release_found);
        assert!(!info.update_available);
        assert_eq!(info.message, "DuckTrack 0.2.0 is up to date.");
    }

    #[test]
    fn reports_no_releases_and_errors_distinctly() {
        let none = build_info("0.2.0".to_string(), Ok(None));
        assert!(!none.release_found);
        assert!(!none.update_available);
        assert_eq!(none.message, "No published releases found on GitHub yet.");

        let failed = build_info("0.2.0".to_string(), Err("GitHub returned HTTP 403.".into()));
        assert!(!failed.release_found);
        assert_eq!(
            failed.message,
            "Could not reach GitHub: GitHub returned HTTP 403."
        );
    }

    #[test]
    fn truncates_long_release_notes() {
        let long = "x".repeat(900);
        let info = build_info("0.1.0".to_string(), Ok(Some(release_json("v0.2.0", &long))));
        let notes = info.notes.unwrap();
        assert_eq!(notes.chars().count(), 401); // 400 + ellipsis
        assert!(notes.ends_with('\u{2026}'));
    }

    #[test]
    fn empty_release_body_yields_no_notes() {
        let info = build_info("0.1.0".to_string(), Ok(Some(release_json("v0.2.0", "   "))));
        assert!(info.notes.is_none());
        assert!(info.update_available);
    }

    /// Live check against the real GitHub API. Ignored by default so CI stays
    /// offline-safe; run with: cargo test -- --ignored --nocapture
    #[test]
    #[ignore = "hits the network"]
    fn live_github_check() {
        let fetched = fetch_release();
        match &fetched {
            Ok(Some(_)) => {}
            other => println!("live check: no release yet -> {other:?}"),
        }
        for current in ["0.1.0", "0.2.0"] {
            let info = build_info(current.to_string(), fetched.clone());
            println!(
                "live: current={current} latest={:?} update_available={} message={}",
                info.latest, info.update_available, info.message
            );
        }
    }
}
