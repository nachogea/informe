//! Per-user local document handoff. Success means a valid GPUI frame rendered.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
const WAIT: Duration = Duration::from_secs(20);
#[derive(Serialize, Deserialize)]
struct Request {
    path: PathBuf,
}
#[derive(Serialize, Deserialize)]
struct Response {
    error: Option<String>,
}
pub struct OpenRequest {
    pub path: PathBuf,
    pub reply: mpsc::Sender<Result<(), String>>,
}
pub struct Server {
    _lock: File,
    socket: PathBuf,
    stop: Arc<AtomicBool>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = fs::remove_file(&self.socket);
    }
}
pub fn runtime_dir() -> PathBuf {
    app_support_dir("Informe")
}
pub fn legacy_runtime_dir() -> PathBuf {
    app_support_dir("Artifact Preview")
}
fn app_support_dir(name: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library/Application Support")
        .join(name)
}
fn receive<T: serde::de::DeserializeOwned>(stream: &mut UnixStream) -> Result<T, String> {
    let mut bytes = Vec::new();
    let mut byte = [0];
    while bytes.len() < 65536 {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                bytes.push(byte[0]);
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    if bytes.len() >= 65536 {
        return Err("IPC message exceeds 64 KiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn send<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    stream.write_all(&bytes).map_err(|e| e.to_string())
}
impl Server {
    pub fn bind(dir: &Path) -> Result<Option<(Self, mpsc::Receiver<OpenRequest>)>, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(dir.join("instance.lock"))
            .map_err(|e| e.to_string())?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        }
        let socket = dir.join("runtime.sock");
        let _ = fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let mut stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(15));
                        continue;
                    }
                    Err(_) => break,
                };
                let _ = stream.set_read_timeout(Some(WAIT));
                let _ = stream.set_write_timeout(Some(WAIT));
                let result = receive::<Request>(&mut stream).and_then(|request| {
                    let path = request.path.canonicalize().map_err(|e| e.to_string())?;
                    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
                    crate::artifact::Artifact::parse_document(&bytes, &path)?;
                    let (reply, response) = mpsc::channel();
                    tx.send(OpenRequest { path, reply })
                        .map_err(|_| "App closed".to_string())?;
                    response
                        .recv_timeout(WAIT)
                        .map_err(|_| "Document did not render before the deadline".to_string())?
                });
                let _ = send(
                    &mut stream,
                    &Response {
                        error: result.err(),
                    },
                );
            }
        });
        Ok(Some((
            Self {
                _lock: lock,
                socket,
                stop,
            },
            rx,
        )))
    }
}
fn connect(dir: &Path, path: &Path) -> Result<(), String> {
    let mut stream = UnixStream::connect(dir.join("runtime.sock")).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(WAIT))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(WAIT))
        .map_err(|e| e.to_string())?;
    send(
        &mut stream,
        &Request {
            path: path.to_owned(),
        },
    )?;
    let response: Response = receive(&mut stream)?;
    response.error.map_or(Ok(()), Err)
}
pub fn open(path: &Path) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    crate::artifact::Artifact::parse_document(&fs::read(&path).map_err(|e| e.to_string())?, &path)?;
    let dir = runtime_dir();
    if UnixStream::connect(dir.join("runtime.sock")).is_ok() {
        return connect(&dir, &path);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // LaunchServices brings bundled builds forward; bare binaries also work in development.
    let bundle = exe
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|p| p.extension().is_some_and(|e| e == "app"));
    let mut command = if let Some(bundle) = bundle {
        let mut c = std::process::Command::new("/usr/bin/open");
        // A legacy build with the same bundle ID may be running without IPC.
        // Launch this build; the server lock resolves concurrent launch attempts.
        c.arg("-n").arg("-a").arg(bundle).arg("--args").arg(&path);
        c
    } else {
        let mut c = std::process::Command::new(exe);
        c.arg(&path);
        c
    };
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Launch failed: {e}"))?;
    let start = Instant::now();
    while start.elapsed() < WAIT {
        if UnixStream::connect(dir.join("runtime.sock")).is_ok() {
            return connect(&dir, &path);
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())?
            && !status.success()
        {
            return Err(format!("App launch exited with {status}"));
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Err("App did not start before the deadline".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_waits_for_render_ack_and_lock_recovers() {
        let dir = std::env::temp_dir().join(format!("artifact-ipc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (server, rx) = Server::bind(&dir).unwrap().unwrap();
        assert!(Server::bind(&dir).unwrap().is_none());
        let path = dir.join("test.json");
        fs::write(&path, include_bytes!("../examples/dashboard.json")).unwrap();
        let client_dir = dir.clone();
        let client_path = path.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let client = std::thread::spawn(move || {
            done_tx.send(connect(&client_dir, &client_path)).unwrap();
        });
        let request = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(request.path, path.canonicalize().unwrap());
        assert!(done_rx.try_recv().is_err());
        request.reply.send(Ok(())).unwrap();
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .is_ok()
        );
        client.join().unwrap();
        fs::write(&path, b"invalid").unwrap();
        assert!(connect(&dir, &path).unwrap_err().contains("expected"));
        drop(server);
        assert!(Server::bind(&dir).unwrap().is_some());
        let _ = fs::remove_dir_all(dir);
    }
}
