// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft`: the command-line front end (docs/specs/commands.md).

use rulercraft_mesh::{ImportLimits, Unit, read_mesh};
use std::process::ExitCode;

const USAGE: &str = "Usage: rulercraft info <mesh.stl|mesh.ply> [--units mm|cm|m|in]";

fn number(v: f32) -> String {
    format!("{v}")
}

fn run(args: &[String]) -> Result<String, String> {
    let [command, path, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    if command != "info" {
        return Err(format!("unknown command `{command}`\n{USAGE}"));
    }
    let unit = match rest {
        [] => Unit::Mm,
        [flag, u] if flag == "--units" => {
            Unit::parse(u).ok_or_else(|| format!("unknown unit `{u}`: use mm, cm, m or in"))?
        }
        _ => return Err(USAGE.into()),
    };
    let format = rulercraft_mesh::io::format_name(path).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let mesh = read_mesh(path, &bytes, unit, ImportLimits::default())
        .map_err(|e| format!("{path}: {e}"))?;
    let b = mesh.bounds().ok_or("empty mesh")?;
    let xyz = |p: [f32; 3]| format!("[{},{},{}]", number(p[0]), number(p[1]), number(p[2]));
    Ok(format!(
        "{{\"format\":\"{format}\",\"vertices\":{},\"triangles\":{},\"degenerate_dropped\":{},\"bounds\":{{\"min\":{},\"max\":{}}}}}",
        mesh.positions.len(),
        mesh.triangles.len(),
        mesh.degenerate_dropped,
        xyz(b.min),
        xyz(b.max)
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rulercraft: {e}");
            ExitCode::from(2)
        }
    }
}
