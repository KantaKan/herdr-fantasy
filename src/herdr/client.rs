//! herdr socket API: newline-delimited JSON (SPEC §3.2). On Unix it's a Unix-domain
//! socket; on Windows herdr serves a named pipe called `\\.\pipe\<socket path>` (the
//! file at the socket path only holds `pid:nonce`). Blocking I/O on its own thread.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, bail};
use serde_json::{Value, json};

#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Stream = std::fs::File;

#[cfg(unix)]
fn open(path: &std::path::Path) -> std::io::Result<Stream> {
    Stream::connect(path)
}

#[cfg(windows)]
fn open(path: &std::path::Path) -> std::io::Result<Stream> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let pipe = format!(r"\\.\pipe\{}", path.display());
    let mut tries = 0;
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe)
        {
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && tries < 20 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            r => return r,
        }
    }
}

/// `$HERDR_SOCKET_PATH`, else the platform default.
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HERDR_SOCKET_PATH") {
        return p.into();
    }
    if cfg!(windows) {
        let appdata = std::env::var_os("APPDATA").unwrap_or_default();
        PathBuf::from(appdata).join("herdr").join("herdr.sock")
    } else {
        directories::BaseDirs::new()
            .map(|d| d.home_dir().join(".config/herdr/herdr.sock"))
            .unwrap_or_else(|| "herdr.sock".into())
    }
}

/// One connection per request: herdr's Windows named pipe closes after each response,
/// and a fresh local connection is cheap on every platform.
pub struct Client {
    path: PathBuf,
    /// The connection opened by `connect`, used by the first request.
    first: Option<Stream>,
    next_id: u64,
}

impl Client {
    /// Fails fast if herdr isn't reachable.
    pub fn connect() -> anyhow::Result<Self> {
        let path = socket_path();
        let s = open(&path).with_context(|| format!("connect {}", path.display()))?;
        Ok(Client {
            path,
            first: Some(s),
            next_id: 0,
        })
    }

    /// One request → its `result`. A herdr error response becomes `Err`.
    pub fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let s = match self.first.take() {
            Some(s) => s,
            None => open(&self.path).with_context(|| format!("connect {}", self.path.display()))?,
        };
        self.next_id += 1;
        let id = format!("hf-{}", self.next_id);
        let mut line = json!({ "id": id, "method": method, "params": params }).to_string();
        line.push('\n');
        let mut writer = s.try_clone()?;
        writer.write_all(line.as_bytes())?;
        let mut reader = BufReader::new(s);
        loop {
            let mut buf = String::new();
            if reader.read_line(&mut buf)? == 0 {
                bail!("herdr closed the connection");
            }
            let mut msg: Value = serde_json::from_str(&buf)?;
            if msg["id"] != id {
                continue; // not ours; skip
            }
            if let Some(err) = msg.get("error") {
                bail!("herdr {method}: {err}");
            }
            return Ok(msg["result"].take());
        }
    }
}
