// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rulercraft`: the command-line front end (docs/specs/commands.md).

mod deviation_cli;

use rulercraft_mesh::commands::{FitKind, FitRequest, Fitted, Selection, fit};
use rulercraft_mesh::fit::Clip;
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::segment::{join_strips, segment_cylinders, segment_planes};
use rulercraft_mesh::{ImportLimits, Mesh, Unit, read_mesh};
use serde_json::{Value, json};
use std::process::ExitCode;

const USAGE: &str = "Usage:
  rulercraft deviation <measured> <reference> [--units U] [--reference-units U] [--tolerance MM] [--offset N] [--limit N]
  rulercraft project create <project.json> <mesh> [--units U]
  rulercraft project fit <project.json> <id> <plane|sphere|cylinder> [fit options]
  rulercraft project info <project.json>
  rulercraft info <mesh> [--units mm|cm|m|in]
  rulercraft segment <mesh> [--kind planes|cylinders|strips] [--units mm|cm|m|in]
  rulercraft thickness <mesh> [selection options] [--offset N] [--limit N] [--max-distance MM]
  rulercraft fit <plane|sphere|cylinder> <mesh> [--units U] [--box X Y Z X Y Z] [--near X Y Z R]
                 [--facing DX DY DZ] [--within DEG] [--sigma 0|1|2|3] [--known-diameter D]
Meshes: .stl .ply .obj .3mf. Lengths in mm (after --units), angles in degrees. Selection filters combine;
with none the whole mesh is used. --within defaults to 30, --sigma to 3 (0 keeps every point).";

fn number(v: f32) -> String {
    format!("{v}")
}

fn load(path: &str, unit: Unit) -> Result<Mesh, String> {
    rulercraft_mesh::io::format_name(path).map_err(|e| e.to_string())?;
    let bytes = limited_read(path, ImportLimits::default().max_bytes)?;
    read_mesh(path, &bytes, unit, ImportLimits::default()).map_err(|e| format!("{path}: {e}"))
}

fn numbers<const N: usize>(
    flag: &str,
    it: &mut std::slice::Iter<'_, String>,
) -> Result<[f64; N], String> {
    let mut out = [0.0; N];
    for v in &mut out {
        *v = it
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| format!("{flag} needs {N} numbers"))?;
    }
    Ok(out)
}

fn fit_request(kind: &str, rest: &[String]) -> Result<(Unit, FitRequest), String> {
    let kind = match kind {
        "plane" => FitKind::Plane,
        "sphere" => FitKind::Sphere,
        "cylinder" => FitKind::Cylinder,
        other => {
            return Err(format!(
                "unknown fit kind `{other}`: use plane, sphere or cylinder"
            ));
        }
    };
    let (mut unit, mut selection, mut clip, mut known) =
        (Unit::Mm, Selection::default(), Clip::Sigma(3), None);
    let (mut facing, mut within) = (None, None);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--units" => {
                let u = it.next().ok_or("--units needs mm, cm, m or in")?;
                unit = Unit::parse(u)
                    .ok_or_else(|| format!("unknown unit `{u}`: use mm, cm, m or in"))?;
            }
            "--box" => {
                let v = numbers::<6>(flag, &mut it)?;
                selection.in_box = Some(([v[0], v[1], v[2]], [v[3], v[4], v[5]]));
            }
            "--near" => {
                let v = numbers::<4>(flag, &mut it)?;
                selection.near = Some(([v[0], v[1], v[2]], v[3]));
            }
            "--facing" => facing = Some(numbers::<3>(flag, &mut it)?),
            "--within" => within = Some(numbers::<1>(flag, &mut it)?[0]),
            "--sigma" => {
                clip = match numbers::<1>(flag, &mut it)?[0] {
                    0.0 => Clip::None,
                    k if k == k.trunc() && (1.0..=3.0).contains(&k) => Clip::Sigma(k as u8),
                    _ => return Err("--sigma must be 0, 1, 2 or 3".into()),
                }
            }
            "--known-diameter" => known = Some(numbers::<1>(flag, &mut it)?[0]),
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    if within.is_some() && facing.is_none() {
        return Err("--within requires --facing".into());
    }
    selection.facing = facing.map(|d| (d, within.unwrap_or(30.0).to_radians()));
    let request = FitRequest {
        kind,
        selection,
        clip,
        known_diameter: known,
    };
    Ok((unit, request))
}

fn fit_command(args: &[String]) -> Result<String, String> {
    let [kind, path, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let (unit, request) = fit_request(kind, rest)?;
    let mesh = load(path, unit)?;
    let graph = MeshGraph::build(&mesh);
    let out = fit(&mesh, &graph, &request).map_err(|e| e.to_string())?;
    let fitted: Value = match out.fitted {
        Fitted::Plane(p) => json!({
            "kind": "plane", "center": p.center, "normal": p.normal, "basis_u": p.basis_u, "basis_v": p.basis_v,
            "span_u": p.span_u, "span_v": p.span_v, "sigma": p.sigma, "form_error": p.form_error,
            "min_distance": p.min_distance, "max_distance": p.max_distance,
            "used": p.used, "offered": p.offered, "rounds": p.rounds,
        }),
        Fitted::Sphere(s) => json!({
            "kind": "sphere", "center": s.center, "radius": s.radius, "diameter": s.diameter, "sigma": s.sigma,
            "form_error": s.form_error, "min_residual": s.min_residual, "max_residual": s.max_residual,
            "used": s.used, "offered": s.offered, "rounds": s.rounds, "cap_angle_deg": s.cap_angle_deg,
            "free_diameter": s.free_diameter,
        }),
        Fitted::Cylinder(c) => json!({
            "kind": "cylinder", "axis_point": c.axis_point, "axis": c.axis, "radius": c.radius,
            "diameter": c.diameter, "length": c.length, "coverage_deg": c.coverage_deg, "sigma": c.sigma,
            "form_error": c.form_error, "min_residual": c.min_residual, "max_residual": c.max_residual,
            "used": c.used, "offered": c.offered, "rounds": c.rounds, "assumed_diameter": c.assumed_diameter,
        }),
    };
    Ok(json!({ "mesh": path, "selected": out.selected, "fit": fitted }).to_string())
}

fn thickness_command(args: &[String]) -> Result<String, String> {
    use rulercraft_mesh::commands::select;
    use rulercraft_mesh::measure::thickness::{ThicknessOptions, wall_thickness};

    let [path, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let (mut offset, mut limit) = (0usize, 256usize);
    let mut options = ThicknessOptions::default();
    let mut selection_args = Vec::new();
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--offset" | "--limit" => {
                let value = it
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| format!("{flag} needs a nonnegative integer"))?;
                if flag == "--offset" {
                    offset = value;
                } else {
                    limit = value;
                }
            }
            "--max-distance" => {
                let value = numbers::<1>(flag, &mut it)?[0];
                if !value.is_finite() || value > ThicknessOptions::default().max_distance {
                    return Err("max_distance must be finite and at most 1000000 mm".into());
                }
                options.max_distance = value;
            }
            "--units" | "--box" | "--near" | "--facing" | "--within" => {
                let count = match flag.as_str() {
                    "--box" => 6,
                    "--near" => 4,
                    "--facing" => 3,
                    _ => 1,
                };
                selection_args.push(flag.clone());
                for _ in 0..count {
                    selection_args.push(
                        it.next()
                            .ok_or_else(|| format!("{flag} needs {count} values"))?
                            .clone(),
                    );
                }
            }
            other => return Err(format!("unknown thickness option `{other}`")),
        }
    }
    if limit > 1024 {
        return Err("thickness page limit must be at most 1024".into());
    }
    let (unit, request) = fit_request("plane", &selection_args)?;
    let mesh = load(path, unit)?;
    let graph = MeshGraph::build(&mesh);
    let ids = select(&mesh, &graph, &request.selection).map_err(|e| e.to_string())?;
    let page: Vec<_> = ids.iter().skip(offset).take(limit).copied().collect();
    let samples =
        wall_thickness(&mesh, &graph.vertex_normals, &page, options).map_err(|e| e.to_string())?;
    let samples: Vec<_> = samples
        .iter()
        .map(|s| json!({"vertex":s.vertex,"distance":s.distance}))
        .collect();
    Ok(json!({"total":ids.len(),"offset":offset,"returned":page.len(),
        "truncated":offset.saturating_add(page.len()) < ids.len(),"samples":samples,
        "convention":"Directional distance along inward vertex normals; requires consistent outward winding. Null means no hit."}).to_string())
}

fn segment_command(args: &[String]) -> Result<String, String> {
    let [path, options @ ..] = args else {
        return Err(USAGE.into());
    };
    let (mut kind, mut unit) = ("cylinders", Unit::Mm);
    let mut it = options.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--kind" => {
                kind = it
                    .next()
                    .ok_or("--kind needs planes, cylinders or strips")?;
                if !matches!(kind, "planes" | "cylinders" | "strips") {
                    return Err("segment kind must be planes, cylinders or strips".into());
                }
            }
            "--units" => {
                let value = it.next().ok_or("--units needs mm, cm, m or in")?;
                unit = Unit::parse(value)
                    .ok_or_else(|| format!("unknown unit `{value}`: use mm, cm, m or in"))?;
            }
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let mesh = load(path, unit)?;
    let graph = MeshGraph::build(&mesh);
    let (regions, unassigned) = match kind {
        "planes" => {
            let segmented = segment_planes(&mesh, &graph, Default::default());
            let regions = segmented.regions.iter().map(|r| json!({
                "triangle_count": r.triangles.len(), "vertex_count": r.vertices.len(), "area": r.area,
                "fit": {"center": r.plane.center, "normal": r.plane.normal,
                    "span_u": r.plane.span_u, "span_v": r.plane.span_v, "form_error": r.plane.form_error}
            })).collect::<Vec<_>>();
            (regions, segmented.unassigned)
        }
        "cylinders" => {
            let segmented = segment_cylinders(&mesh, &graph, Default::default());
            let regions = segmented.regions.iter().map(|r| json!({
                "triangle_count": r.triangles.len(), "vertex_count": r.vertices.len(), "area": r.area,
                "fit": {"axis_point": r.cylinder.axis_point, "axis": r.cylinder.axis,
                    "radius": r.cylinder.radius, "diameter": r.cylinder.diameter,
                    "coverage_deg": r.cylinder.coverage_deg, "form_error": r.cylinder.form_error}
            })).collect::<Vec<_>>();
            (regions, segmented.unassigned)
        }
        "strips" => {
            let planes = segment_planes(&mesh, &graph, Default::default());
            let surfaces = join_strips(&mesh, &graph, &planes, Default::default());
            let unassigned = mesh
                .triangles
                .len()
                .saturating_sub(surfaces.iter().map(|r| r.triangles.len()).sum::<usize>());
            let regions = surfaces.iter().map(|r| json!({
                "triangle_count": r.triangles.len(), "vertex_count": r.vertices.len(),
                "strip_count": r.strips.len(), "area": r.area,
                "fit": {"axis_point": r.cylinder.axis_point, "axis": r.cylinder.axis,
                    "radius": r.cylinder.radius, "diameter": r.cylinder.diameter,
                    "coverage_deg": r.cylinder.coverage_deg, "form_error": r.cylinder.form_error}
            })).collect::<Vec<_>>();
            (regions, unassigned)
        }
        _ => return Err("segment kind must be planes, cylinders or strips".into()),
    };
    Ok(json!({"mesh":path,"kind":kind,"regions":regions,"unassigned":unassigned}).to_string())
}

fn run(args: &[String]) -> Result<String, String> {
    let [command, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    if command == "deviation" {
        return deviation_cli::run(rest);
    }
    if command == "project" {
        return project_command(rest);
    }
    if command == "fit" {
        return fit_command(rest);
    }
    if command == "segment" {
        return segment_command(rest);
    }
    if command == "thickness" {
        return thickness_command(rest);
    }
    let [path, rest @ ..] = rest else {
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
    let mesh = load(path, unit)?;
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

fn limited_read(path: &str, max: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    if file.metadata().map_err(|e| format!("{path}: {e}"))?.len() > max as u64 {
        return Err(format!("{path}: byte limit exceeded ({max})"));
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{path}: {e}"))?;
    if bytes.len() > max {
        return Err(format!("{path}: byte limit exceeded ({max})"));
    }
    Ok(bytes)
}

fn write_project(path: &str, bytes: &[u8], create: bool) -> Result<(), String> {
    use std::io::Write;
    // Create-new protects our temporary path. Hard-link creation refuses to replace an existing
    // destination; subsequent edits atomically rename over the project only after writing finishes.
    let temporary = format!("{path}.{}.tmp", std::process::id());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("{temporary}: {e}"))?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        if create {
            std::fs::hard_link(&temporary, path)?;
        } else {
            std::fs::rename(&temporary, path)?;
        }
        Ok(())
    })();
    drop(file);
    let _ = std::fs::remove_file(&temporary);
    result.map_err(|e| format!("{path}: {e}"))
}

fn project_command(args: &[String]) -> Result<String, String> {
    use rulercraft_mesh::project::{Project, ProjectLimits};
    let limits = ProjectLimits::default();
    let [operation, path, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let (project, changed) = match operation.as_str() {
        "create" => {
            let [mesh, options @ ..] = rest else {
                return Err(USAGE.into());
            };
            let unit = match options {
                [] => Unit::Mm,
                [flag, u] if flag == "--units" => {
                    Unit::parse(u).ok_or("source units must be mm, cm, m or in")?
                }
                _ => return Err(USAGE.into()),
            };
            let bytes = limited_read(mesh, limits.import.max_bytes)?;
            let project = Project::new(mesh, &bytes, unit, limits).map_err(|e| e.to_string())?;
            write_project(path, &project.encode().map_err(|e| e.to_string())?, true)?;
            (project, Vec::new())
        }
        "info" | "fit" => {
            let reopened = Project::decode(&limited_read(path, limits.max_bytes)?, limits)
                .map_err(|e| e.to_string())?;
            let mut project = reopened.project;
            if operation == "fit" {
                let [id, kind, options @ ..] = rest else {
                    return Err(USAGE.into());
                };
                if options.iter().any(|s| s == "--units") {
                    return Err("project units come from its saved source".into());
                }
                let (_, recipe) = fit_request(kind, options)?;
                project.add(id, recipe).map_err(|e| e.to_string())?;
                write_project(path, &project.encode().map_err(|e| e.to_string())?, false)?;
            } else if !rest.is_empty() {
                return Err(USAGE.into());
            }
            (project, reopened.changed)
        }
        _ => return Err(USAGE.into()),
    };
    Ok(json!({"project":path,"vertices":project.mesh().positions.len(),
              "triangles":project.mesh().triangles.len(),"elements":project.elements(),"changed":changed}).to_string())
}
