use crate::model::{App, AppError, Source};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

/// Response envelope for a single snap (`/v2/snaps/<name>`).
#[derive(Debug, Deserialize)]
struct SingleSnapResponse {
    result: SingleSnapEntry,
}

#[derive(Debug, Deserialize)]
struct SingleSnapEntry {
    #[serde(default)]
    description: Option<String>,
}

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

/// Extract the JSON body from a raw snapd HTTP response.
/// Non-2xx statuses become typed errors; chunked bodies are rejected clearly
/// (snapd answers HTTP/1.0 requests connection-close, so chunking is unexpected).
fn extract_body(raw: &str) -> Result<String, AppError> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| AppError::Backend("snapd: malformed HTTP response".into()))?;
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or("");
    let code = status_line.split_whitespace().nth(1).unwrap_or("");
    if !code.starts_with('2') {
        return Err(match code {
            "401" | "403" => AppError::PermissionDenied(format!("snapd: {status_line}")),
            _ => AppError::SourceUnavailable(format!("snapd HTTP status: {status_line}")),
        });
    }
    if lines.any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    }) {
        return Err(AppError::Backend("snapd: chunked response unsupported".into()));
    }
    Ok(body.to_string())
}

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
    // HTTP/1.0 + no keep-alive: snapd closes the socket, so the bounded read sees
    // EOF. Cap the read so a misbehaving snapd can't OOM the app; 8 MiB is far
    // beyond any legitimate /v2/snaps body.
    const MAX_BODY: u64 = 8 * 1024 * 1024;
    Read::take(stream, MAX_BODY)
        .read_to_string(&mut raw)
        .map_err(|e| AppError::Backend(format!("snapd read: {e}")))?;
    extract_body(&raw)
}

/// List installed snap apps from the live socket.
pub fn list() -> Result<Vec<App>, AppError> {
    let body = snapd_get("/v2/snaps")?;
    parse_snaps(&body)
}

/// Fetch the long `description` field for a single snap. Returns None on any error.
pub fn get_snap_description(snap_name: &str) -> Option<String> {
    let path = format!("/v2/snaps/{snap_name}");
    let body = snapd_get(&path).ok()?;
    let resp: SingleSnapResponse = serde_json::from_str(&body).ok()?;
    resp.result.description.filter(|s| !s.is_empty())
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
    fn extract_body_returns_body_on_200() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"x\":1}";
        assert_eq!(extract_body(raw).unwrap(), "{\"x\":1}");
    }

    #[test]
    fn extract_body_permission_denied_on_403() {
        let raw = "HTTP/1.1 403 Forbidden\r\n\r\n{...}";
        assert!(matches!(extract_body(raw), Err(AppError::PermissionDenied(_))));
    }

    #[test]
    fn extract_body_source_unavailable_on_500() {
        let raw = "HTTP/1.1 500 Internal Server Error\r\n\r\n{}";
        assert!(matches!(extract_body(raw), Err(AppError::SourceUnavailable(_))));
    }

    #[test]
    fn extract_body_rejects_chunked() {
        let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        assert!(matches!(extract_body(raw), Err(AppError::Backend(_))));
    }

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
