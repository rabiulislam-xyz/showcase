use crate::model::{App, AppError, Source};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SnapdResponse {
    result: Vec<SnapEntry>,
}

#[derive(Debug, Deserialize)]
struct SnapEntry {
    name: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "type", default)]
    snap_type: String,
    #[serde(rename = "installed-size", default)]
    installed_size: u64,
    #[serde(rename = "install-date", default)]
    install_date: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    publisher: Option<Publisher>,
}

#[derive(Debug, Deserialize)]
struct Publisher {
    #[serde(rename = "display-name", default)]
    display_name: Option<String>,
}

/// Parse a snapd `/v2/snaps` JSON body into apps, keeping only `type == "app"`.
pub fn parse_snaps(body: &str) -> Result<Vec<App>, AppError> {
    let resp: SnapdResponse = serde_json::from_str(body)
        .map_err(|e| AppError::Backend(format!("snapd json: {e}")))?;
    let apps = resp
        .result
        .into_iter()
        .filter(|s| s.snap_type == "app")
        .map(|s| App {
            uid: App::make_uid(Source::Snap, &s.name),
            source: Source::Snap,
            name: s.name.clone(),
            summary: s.summary,
            description: None,
            version: (!s.version.is_empty()).then(|| s.version.clone()),
            icon_path: None,
            size_bytes: (s.installed_size > 0).then_some(s.installed_size),
            install_date: s.install_date,
            publisher: s.publisher.and_then(|p| p.display_name),
            categories: Vec::new(),
            exec: None,
            pkg_ref: s.name.clone(),
            removable: true,
            protected_reason: None,
        })
        .collect();
    Ok(apps)
}

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const SOCKET: &str = "/run/snapd.socket";

/// GET a snapd REST path over the unix socket and return the response body.
/// Minimal blocking HTTP/1.0 client (snapd speaks HTTP over the socket).
pub fn snapd_get(path: &str) -> Result<String, AppError> {
    let mut stream = UnixStream::connect(SOCKET)
        .map_err(|e| AppError::SourceUnavailable(format!("snapd socket: {e}")))?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| AppError::Backend(format!("snapd write: {e}")))?;
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|e| AppError::Backend(format!("snapd read: {e}")))?;
    // Split headers from body at the first blank line.
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .unwrap_or("")
        .to_string();
    Ok(body)
}

/// List installed snap apps from the live socket.
pub fn list() -> Result<Vec<App>, AppError> {
    let body = snapd_get("/v2/snaps")?;
    parse_snaps(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{
      "type":"sync",
      "result":[
        {"name":"firefox","version":"124.0","type":"app","installed-size":256000000,
         "install-date":"2026-03-01T10:00:00Z","summary":"Web browser",
         "publisher":{"display-name":"Mozilla"}},
        {"name":"core22","version":"20260101","type":"base","installed-size":77000000}
      ]
    }"#;

    #[test]
    fn keeps_only_app_type() {
        let apps = parse_snaps(BODY).unwrap();
        assert_eq!(apps.len(), 1);
        let a = &apps[0];
        assert_eq!(a.uid, "snap:firefox");
        assert_eq!(a.version.as_deref(), Some("124.0"));
        assert_eq!(a.size_bytes, Some(256000000));
        assert_eq!(a.publisher.as_deref(), Some("Mozilla"));
        assert_eq!(a.install_date.as_deref(), Some("2026-03-01T10:00:00Z"));
    }
}
