// dsh-desktop: thin Tauri wrapper around the DeepSeek Harness Web GUI.
//
// Behaviour:
//  - On startup it spawns `dsh web` as a child process and shows a local
//    splash screen while waiting for http://127.0.0.1:3080 to accept
//    connections, then navigates the main window to the DSH web GUI.
//  - On exit it terminates the child (and its process group on Unix).
//  - The URL can be overridden with the DSH_WEB_URL environment variable.

use std::io;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, RunEvent};

/// Holds the `dsh web` child process so it can be killed on exit.
struct DshServer(Mutex<Option<Child>>);

const DSH_DEFAULT_URL: &str = "http://127.0.0.1:3080";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

// ---------------------------------------------------------------------------
// dsh web process management
// ---------------------------------------------------------------------------

fn dsh_url() -> String {
    std::env::var("DSH_WEB_URL").unwrap_or_else(|_| DSH_DEFAULT_URL.to_string())
}

/// "http://127.0.0.1:3080/[...]" -> ("127.0.0.1", 3080)
fn parse_host_port(url: &str) -> (String, u16) {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host_port = rest.split('/').next().unwrap_or(rest);
    match host_port.rsplit_once(':') {
        Some((host, port)) => (host.to_string(), port.parse().unwrap_or(3080)),
        None => (host_port.to_string(), 3080),
    }
}

fn server_ready(url: &str) -> bool {
    let (host, port) = parse_host_port(url);
    TcpStream::connect((host.as_str(), port)).is_ok()
}

fn wait_for_server(url: &str) -> bool {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if server_ready(url) {
            return true;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    server_ready(url)
}

/// Resolve the `dsh` CLI: PATH lookup first, then ~/.local/bin/dsh (the common
/// location for user-level npm global installs).
fn resolve_dsh() -> String {
    if let Ok(output) = Command::new("sh").args(["-c", "command -v dsh"]).output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return path;
        }
    }
    let fallback = format!(
        "{}/.local/bin/dsh",
        std::env::var("HOME").unwrap_or_default()
    );
    eprintln!("`dsh` not found on PATH, falling back to {fallback}");
    fallback
}

fn spawn_dsh() -> io::Result<Child> {
    let mut cmd = Command::new(resolve_dsh());
    cmd.arg("web");

    // In debug builds keep the server logs visible in the launching terminal;
    // in release builds (launched from a desktop entry) silence them.
    if cfg!(debug_assertions) {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    // Own process group so that children of `dsh web` die together with it.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn()
}

fn kill_dsh(child: &mut Child) {
    #[cfg(unix)]
    {
        // process_group(0) made the child a group leader; kill the whole group.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGTERM);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

// ---------------------------------------------------------------------------
// Window navigation
// ---------------------------------------------------------------------------

fn navigate_to(app: &AppHandle, target: &str) {
    let Ok(url) = tauri::Url::parse(target) else {
        eprintln!("invalid navigation target: {target}");
        return;
    };
    // The window may not exist yet right after setup, so retry briefly.
    for _ in 0..50 {
        if let Some(win) = app.get_webview_window("main") {
            if let Err(e) = win.navigate(url.clone()) {
                eprintln!("navigate to {target} failed: {e}");
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A minimal inline error page shown when the DSH server cannot be started.
fn error_page(title: &str, message: &str) -> String {
    fn encode(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    let html = format!(
        r#"<!doctype html><html lang="ja"><head><meta charset="utf-8"><style>
body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#0d1117;color:#e6edf3;font-family:system-ui,sans-serif;}}
.card{{max-width:540px;padding:40px;text-align:center;}}
h1{{font-size:1.15rem;margin:0 0 12px;}}
p{{color:#9da7b3;line-height:1.7;margin:0;}}
code{{background:#161b22;padding:2px 6px;border-radius:4px;}}
</style></head><body><div class="card"><h1>{title}</h1><p>{message}</p></div></body></html>"#
    );
    format!("data:text/html;charset=utf-8,{}", encode(&html))
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let url = dsh_url();
            let handle = app.handle().clone();
            match spawn_dsh() {
                Ok(child) => {
                    app.manage(DshServer(Mutex::new(Some(child))));
                    std::thread::spawn(move || {
                        if wait_for_server(&url) {
                            navigate_to(&handle, &url);
                        } else {
                            eprintln!(
                                "DSH server did not become ready at {url} within {STARTUP_TIMEOUT:?}"
                            );
                            let page = error_page(
                                "DSH サーバーが起動しませんでした",
                                &format!(
                                    "{STARTUP_TIMEOUT:?} 以内に {url} に接続できませんでした。<br>別のターミナルで <code>dsh web</code> が正常に起動するか確認してください。"
                                ),
                            );
                            navigate_to(&handle, &page);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("failed to spawn `dsh web`: {e}");
                    std::thread::spawn(move || {
                        let page = error_page(
                            "起動エラー",
                            &format!(
                                "<code>dsh web</code> を起動できませんでした: {e}<br>DSH CLI (<code>dsh</code>) がインストールされているか確認してください。"
                            ),
                        );
                        navigate_to(&handle, &page);
                    });
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { .. } = event {
            if let Some(state) = app_handle.try_state::<DshServer>() {
                if let Some(mut child) = state.0.lock().unwrap().take() {
                    kill_dsh(&mut child);
                }
            }
        }
    });
}