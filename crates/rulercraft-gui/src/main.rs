// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft-gui [mesh] [--port N] [--no-open]`: the standalone app (docs/specs/gui.md).
use rulercraft_gui::{State, serve};
use std::net::TcpListener;
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex};

const USAGE: &str = "Usage: rulercraft-gui [mesh.stl|ply|obj|3mf] [--units mm|cm|m|in] [--port N] [--no-open]
Starts RulerCraft on this computer only (127.0.0.1) and opens it in your browser. Close the terminal to quit.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut path, mut unit, mut port, mut open) = (None, "mm".to_string(), 0u16, true);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--no-open" => open = false,
            "--units" => unit = it.next().cloned().unwrap_or_default(),
            "--port" => match it.next().and_then(|p| p.parse().ok()) {
                Some(p) => port = p,
                None => return fail("--port needs a number"),
            },
            f if f.starts_with("--") => return fail(&format!("unknown option {f}")),
            f if path.is_none() => path = Some(f.to_string()),
            _ => return fail("only one mesh file may be given"),
        }
    }
    let mut state = State::default();
    if let Some(p) = &path {
        if let Err(e) = state.open_file(p, &unit) {
            return fail(&e);
        }
    }
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => return fail(&format!("cannot listen on 127.0.0.1:{port}: {e}")),
    };
    let addr = match listener.local_addr() {
        Ok(a) => a,
        Err(e) => return fail(&e.to_string()),
    };
    let url = format!("http://127.0.0.1:{}/", addr.port());
    println!("RulerCraft is running at {url}  (this computer only; close this window to quit)");
    if open {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(target_os = "windows") {
            "explorer"
        } else {
            "xdg-open"
        };
        if Command::new(opener).arg(&url).spawn().is_err() {
            println!("Could not open a browser; open the address above yourself.");
        }
    }
    serve(listener, Arc::new(Mutex::new(state)));
    ExitCode::SUCCESS
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("rulercraft-gui: {msg}\n{USAGE}");
    ExitCode::from(2)
}
