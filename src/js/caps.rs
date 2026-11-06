//! Files and HTTP for extension code. Each takes and returns JSON, so the
//! prelude shapes what extension code sees.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchRequest {
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

/// Shared, so repeated calls to one host reuse its connection, and bounded,
/// so a server that stalls ends the request instead of the view's loading.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
});

pub async fn fetch(request: FetchRequest) -> Result<FetchResponse, String> {
    let method = request
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse::<reqwest::Method>()
        .map_err(|error| error.to_string())?;
    let mut builder = CLIENT.request(method, &request.url);
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let response = builder.send().await.map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
        .collect();
    let body = response.text().await.map_err(|error| error.to_string())?;
    Ok(FetchResponse {
        status,
        headers,
        body,
    })
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum FsRequest {
    ReadText { path: String },
    WriteText { path: String, text: String },
    ReadDir { path: String },
    Remove { path: String },
    Exists { path: String },
    Stat { path: String },
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub size: u64,
    pub is_dir: bool,
    pub modified_ms: Option<u64>,
}

/// Runs one file operation and returns its result as JSON.
pub async fn fs(request: FsRequest) -> Result<String, String> {
    tokio::task::spawn_blocking(move || fs_now(request))
        .await
        .map_err(|error| error.to_string())?
}

fn fs_now(request: FsRequest) -> Result<String, String> {
    let json = |value: serde_json::Value| Ok(value.to_string());
    match request {
        FsRequest::ReadText { path } => json(
            std::fs::read_to_string(&path)
                .map_err(|e| failed(&path, e))?
                .into(),
        ),
        FsRequest::WriteText { path, text } => {
            if let Some(parent) = Path::new(&path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| failed(&path, e))?;
            }
            std::fs::write(&path, text).map_err(|e| failed(&path, e))?;
            json(serde_json::Value::Null)
        }
        FsRequest::ReadDir { path } => {
            let mut entries: Vec<Entry> = std::fs::read_dir(&path)
                .map_err(|e| failed(&path, e))?
                .filter_map(Result::ok)
                .map(|entry| Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_dir: entry.file_type().is_ok_and(|kind| kind.is_dir()),
                })
                .collect();
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            serde_json::to_string(&entries).map_err(|error| error.to_string())
        }
        FsRequest::Remove { path } => {
            let target = Path::new(&path);
            let removed = match target.is_dir() {
                true => std::fs::remove_dir_all(target),
                false => std::fs::remove_file(target),
            };
            removed.map_err(|e| failed(&path, e))?;
            json(serde_json::Value::Null)
        }
        FsRequest::Exists { path } => json(Path::new(&path).exists().into()),
        FsRequest::Stat { path } => {
            let metadata = std::fs::metadata(&path).map_err(|e| failed(&path, e))?;
            let stat = Stat {
                size: metadata.len(),
                is_dir: metadata.is_dir(),
                modified_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|since| since.as_millis() as u64),
            };
            serde_json::to_string(&stat).map_err(|error| error.to_string())
        }
    }
}

fn failed(path: &str, error: std::io::Error) -> String {
    format!("{path}: {error}")
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dango-fs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn path(dir: &Path, name: &str) -> String {
        dir.join(name).to_string_lossy().into_owned()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn files_are_written_listed_read_and_removed() {
        let dir = temp();
        let file = path(&dir, "sub/a.txt");

        fs(FsRequest::WriteText {
            path: file.clone(),
            text: "hi".into(),
        })
        .await
        .unwrap();
        let read = fs(FsRequest::ReadText { path: file.clone() })
            .await
            .unwrap();
        let listed = fs(FsRequest::ReadDir {
            path: path(&dir, "sub"),
        })
        .await
        .unwrap();
        let stat = fs(FsRequest::Stat { path: file.clone() }).await.unwrap();
        fs(FsRequest::Remove {
            path: path(&dir, "sub"),
        })
        .await
        .unwrap();
        let exists = fs(FsRequest::Exists { path: file }).await.unwrap();

        assert_eq!(read, r#""hi""#);
        assert_eq!(listed, r#"[{"name":"a.txt","isDir":false}]"#);
        assert!(stat.contains(r#""size":2"#), "{stat}");
        assert_eq!(exists, "false");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_missing_file_names_its_path() {
        let missing = path(&temp(), "nope.txt");

        let error = fs(FsRequest::ReadText {
            path: missing.clone(),
        })
        .await
        .unwrap_err();

        assert!(error.starts_with(&missing), "{error}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetch_returns_status_headers_and_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            let body = r#"{"ok":true}"#;
            let _ = write!(
                stream,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        });

        let response = fetch(FetchRequest {
            url: format!("http://{address}/thing"),
            method: Some("POST".into()),
            headers: BTreeMap::new(),
            body: Some("x".into()),
        })
        .await
        .unwrap();

        assert_eq!(response.status, 201);
        assert_eq!(response.headers["content-type"], "application/json");
        assert_eq!(response.body, r#"{"ok":true}"#);
    }
}
