//! A test BIMCraft plugin: builds a small house with ordinary commands.
//!
//! One call makes the four walls, the floor, a door, a window, a room, a roof and a note. Every step is a command, so the
//! whole call is one undo step and a failure in the middle takes all of it back. It reads the model with the same
//! `elements` command in `check`, which answers with a `report` the window opens on screen.

use bimcraft_plugin_sdk::{Value, execute, host_info, info_log};
use serde_json::json;

fn number(params: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64, String> {
    let value = match params.get(key).filter(|v| !v.is_null()) {
        Some(v) => v.as_f64().ok_or_else(|| format!("`{key}` is a number, not {v}"))?,
        None => default,
    };
    if !value.is_finite() || value < min || value > max {
        return Err(format!("`{key}` is from {min} to {max}, not {value}"));
    }
    Ok(value)
}

fn point(params: &Value, key: &str, default: [f64; 2]) -> Result<[f64; 2], String> {
    let Some(v) = params.get(key).filter(|v| !v.is_null()) else { return Ok(default) };
    let pair = v.as_array().filter(|a| a.len() == 2).ok_or_else(|| format!("`{key}` is [x, y], not {v}"))?;
    let mut out = [0.0; 2];
    for (i, at) in pair.iter().enumerate() {
        let n = at.as_f64().ok_or_else(|| format!("`{key}` is [x, y], not {v}"))?;
        if !n.is_finite() || n.abs() > 1.0e6 {
            return Err(format!("`{key}` is a point in mm, not {v}"));
        }
        out[i] = n;
    }
    Ok(out)
}

/// `build`: walls, floor, door, window, room, roof and a note, all in one command call.
fn build(params: &Value) -> Result<Value, String> {
    let at = point(params, "at", [0.0, 0.0])?;
    let width = number(params, "width", 8000.0, 2000.0, 50_000.0)?;
    let depth = number(params, "depth", 6000.0, 2000.0, 50_000.0)?;
    let room_name = params.get("room").and_then(Value::as_str).unwrap_or("Living Room").to_string();
    let (x, y) = (at[0], at[1]);
    let (x2, y2) = (x + width, y + depth);
    let outline = json!([[x, y], [x2, y], [x2, y2], [x, y2]]);

    info_log(&format!("building a {width} x {depth} mm house at {x}, {y}"));

    // Walls: up to the level above unless a height is given.
    let walls = match params.get("height").filter(|v| !v.is_null()) {
        Some(_) => {
            let height = number(params, "height", 3000.0, 1000.0, 20_000.0)?;
            execute("wall", &json!({"points": outline, "closed": true, "height": height}))?
        }
        None => execute("wall", &json!({"points": outline, "closed": true}))?,
    };
    let floor = execute("floor", &json!({"boundary": outline}))?;
    let door = execute("door", &json!({"at": [x + width / 2.0, y], "mark": "D1"}))?;
    let window = execute("window", &json!({"at": [x2, y + depth / 2.0], "mark": "W1"}))?;
    let room = execute("room", &json!({"point": [x + width / 2.0, y + depth / 2.0], "name": room_name, "number": "101"}))?;
    let roof = execute("roof", &json!({"overhang": 500.0, "slope": 30.0}))?;
    let note = execute(
        "text",
        &json!({
            "text": format!("{room_name}: {width} x {depth} mm\nBuilt by the House Builder plugin"),
            "at": [x + 300.0, y - 800.0]
        }),
    )?;

    info_log("the house is built: one undo step");
    Ok(json!({
        "size": [width, depth],
        "walls": walls,
        "floor": floor,
        "door": door,
        "window": window,
        "room": room,
        "roof": roof,
        "note": note
    }))
}

/// A text in `v` under `key`, or "" when there isn't one.
fn text_in<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

/// A number in `v` under `key`, or none when there isn't one.
fn number_in(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

/// A millimetre measure as the report prints it: 3000 -> "3000", 2100.5 -> "2100.5".
fn mm(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{rounded}")
}

/// Millimetres as metres: 8000 -> "8.00 m".
fn metres(value: f64) -> String {
    format!("{:.2} m", value / 1000.0)
}

/// Square metres as the report prints them: 48 -> "48.0 m2".
fn square_metres(value: f64) -> String {
    format!("{value:.1} m2")
}

/// What a category is called in the report, one of them: `wall` -> `Wall`.
fn category_name(category: &str) -> String {
    let one = match category {
        "wall" => "Wall",
        "floor" => "Floor",
        "ceiling" => "Ceiling",
        "roof" => "Roof",
        "door" => "Door",
        "window" => "Window",
        "room" => "Room",
        "roomseparator" => "Room separator",
        "column" | "structuralcolumn" => "Column",
        "footing" | "stripfooting" => "Footing",
        "beam" => "Beam",
        "stair" => "Stair",
        "ramp" => "Ramp",
        "railing" => "Railing",
        "grid" => "Grid",
        "level" => "Level",
        "mass" => "Mass",
        "shaft" => "Shaft",
        "text" => "Note",
        "line" => "Detail line",
        "region" => "Filled region",
        "dimension" | "dimension3d" => "Dimension",
        "tag" => "Tag",
        "component" => "Component",
        "rebar" => "Rebar",
        "pipe" => "Pipe",
        "duct" => "Duct",
        "wire" => "Wire",
        "cable" => "Cable",
        "cabletray" => "Cable tray",
        "conduit" => "Conduit",
        other => other,
    };
    let mut chars = one.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// The plural of a category's name, for the report's headings: `Wall` -> `Walls`, `Mass` -> `Masses`, `Grid` -> `Grids`.
fn plural(word: &str) -> String {
    let last = word.as_bytes().last().copied();
    let before = word.as_bytes().get(word.len().saturating_sub(2)).copied();
    let vowel = |byte: u8| matches!(byte, b'a' | b'e' | b'i' | b'o' | b'u' | b'A' | b'E' | b'I' | b'O' | b'U');
    if matches!(last, Some(b's' | b'x')) || word.ends_with("sh") || word.ends_with("ch") {
        format!("{word}es")
    } else if last == Some(b'y') && before.is_some_and(|byte| !vowel(byte)) {
        format!("{}ies", &word[..word.len() - 1])
    } else {
        format!("{word}s")
    }
}

/// The elements of `category`, at most `limit` of them, and how many the model has of them: the count is the whole
/// model's, the rows are only the page `limit` allows. None: the command refused, which the report says, rather than
/// the whole call failing.
fn element_rows(category: &str, limit: u64) -> Option<(u64, Vec<Value>)> {
    let reply = execute("elements", &json!({"category": category, "limit": limit})).ok()?;
    let count = reply.get("count").and_then(Value::as_u64).unwrap_or(0);
    let rows = reply.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    Some((count, rows))
}

/// How many elements of `category` stand on the level named `level`: only the count, with no rows to carry.
fn level_count(category: &str, level: &str) -> Option<u64> {
    let reply = execute("elements", &json!({"category": category, "level": level, "limit": 0})).ok()?;
    reply.get("count").and_then(Value::as_u64)
}

/// What the rows' `type`s are, each with how many rows are of it: `Basic ×4`.
fn types_of(rows: &[Value]) -> Vec<String> {
    let mut types: Vec<(String, u64)> = Vec::new();
    for row in rows {
        let raw = text_in(row, "type");
        let name = if raw.is_empty() { "unnamed" } else { raw };
        match types.iter().position(|(found, _)| found.as_str() == name) {
            Some(at) => types[at].1 += 1,
            None => types.push((name.to_string(), 1)),
        }
    }
    types.into_iter().map(|(name, n)| format!("{name} ×{n}")).collect()
}

/// `check`: the model's composition as a person reads it — its levels, its elements by category and by level, the rooms
/// with their areas, the walls, the openings, the floors, the roofs, the notes, the library, the warnings and the undo
/// history — as a `report` the window opens on screen. It only reads the model, so it makes no undo step.
fn check(_: &Value) -> Result<Value, String> {
    // `brief`: the library's size instead of its contents, which are most of the answer and cost fuel to read.
    let model = execute("inspect", &json!({"brief": true}))?;
    let info = host_info().unwrap_or(Value::Null);
    let mut out: Vec<String> = Vec::new();

    // What the report is of, and what the model measures in.
    let project = text_in(&model, "name");
    out.push(format!("MODEL REPORT - {}", if project.is_empty() { "an untitled project" } else { project }));
    let (app, version) = (text_in(&info, "app"), text_in(&info, "version"));
    let mut head: Vec<String> = Vec::new();
    if !app.is_empty() {
        head.push(if version.is_empty() { app.to_string() } else { format!("{app} {version}") });
    }
    let (standard, language) = (text_in(&model, "standard"), text_in(&model, "language"));
    if !standard.is_empty() {
        head.push(format!("standard {standard}"));
    }
    if !language.is_empty() {
        head.push(format!("language {language}"));
    }
    let area_unit = text_in(&model, "room_area");
    if !area_unit.is_empty() {
        head.push(format!("areas in {area_unit}"));
    }
    head.push("lengths in mm".to_string());
    out.push(head.join(" · "));
    if let Some(path) = model.get("path").and_then(Value::as_str).filter(|p| !p.is_empty()) {
        out.push(format!("file: {path}"));
    }

    // Levels, with their elevations.
    let levels: Vec<Value> = model.get("levels").and_then(Value::as_array).cloned().unwrap_or_default();
    out.push(String::new());
    out.push(format!("LEVELS ({})", levels.len()));
    if levels.is_empty() {
        out.push("  the model has no levels".to_string());
    }
    for level in &levels {
        let elevation = mm(number_in(level, "elevation").unwrap_or(0.0));
        out.push(format!("  {:<30} {:>10} mm", text_in(level, "name"), elevation));
    }

    // Every category the model has, the most numerous first.
    let counts = model.get("counts").cloned().unwrap_or(Value::Null);
    let mut categories: Vec<(String, u64)> = counts
        .as_object()
        .map(|c| c.iter().map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0))).filter(|(_, n)| *n > 0).collect())
        .unwrap_or_default();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let total: u64 = categories.iter().map(|(_, n)| *n).sum();
    out.push(String::new());
    out.push(if categories.is_empty() { "ELEMENTS (none)".to_string() } else { format!("ELEMENTS ({total} in all)") });
    for (category, n) in &categories {
        out.push(format!("  {:<30} {:>6}", plural(&category_name(category)), n));
    }

    // Every command's answer costs fuel by its size, so a big model gets shorter lists; the counts above are
    // always whole, the rows are only a page of them.
    let limit: u64 = if total <= 40 { 60 } else { 8 };

    // What each level holds, of the categories that have something somewhere.
    if !levels.is_empty() && !categories.is_empty() {
        out.push(String::new());
        out.push("BY LEVEL".to_string());
        let pairs = levels.len() * categories.len();
        if pairs > 40 {
            out.push(format!("  {pairs} level and category pairs: too many to count one by one; ELEMENTS has the totals"));
        } else {
            for level in &levels {
                let name = text_in(level, "name");
                let elevation = mm(number_in(level, "elevation").unwrap_or(0.0));
                out.push(format!("  {name} ({elevation} mm)"));
                let mut here: Vec<String> = Vec::new();
                let mut refused = false;
                for (category, _) in &categories {
                    match level_count(category, name) {
                        Some(n) if n > 0 => here.push(format!("{} {n}", plural(&category_name(category)))),
                        Some(_) => {}
                        None => refused = true,
                    }
                }
                if here.is_empty() {
                    out.push(if refused { "    could not be counted".to_string() } else { "    nothing".to_string() });
                } else {
                    out.push(format!("    {}", here.join(" · ")));
                }
            }
        }
    }

    // Rooms: their numbers, their names and their areas.
    if let Some((count, rows)) = element_rows("room", limit) {
        out.push(String::new());
        out.push(format!("ROOMS ({count})"));
        if rows.is_empty() {
            out.push("  none".to_string());
        }
        let mut area = 0.0;
        for row in &rows {
            let here = number_in(row, "area_m2").unwrap_or(0.0);
            area += here;
            let state = if row.get("enclosed").and_then(Value::as_bool) == Some(true) { "" } else { "  NOT enclosed" };
            out.push(format!("  {:<8} {:<26} {:>9.1} m2{state}", text_in(row, "number"), text_in(row, "name"), here));
        }
        let rest = count.saturating_sub(rows.len() as u64);
        if rest > 0 {
            out.push(format!("  … and {rest} more"));
        } else if !rows.is_empty() {
            out.push(format!("  total {}", square_metres(area)));
        }
    }

    // Walls: how long they run in all, how tall they are, and what they are of.
    if let Some((count, rows)) = element_rows("wall", limit) {
        out.push(String::new());
        out.push(format!("WALLS ({count})"));
        if rows.is_empty() {
            out.push("  none".to_string());
        }
        let mut heights: Vec<String> = Vec::new();
        for row in &rows {
            let height = mm(number_in(row, "height").unwrap_or(0.0));
            if !heights.contains(&height) {
                heights.push(height);
            }
        }
        let rest = count.saturating_sub(rows.len() as u64);
        let mut facts: Vec<String> = Vec::new();
        if !heights.is_empty() {
            facts.push(format!("height {} mm", heights.join(" / ")));
        }
        // The length is of the walls listed: only a whole list has a whole length.
        if rest == 0 && !rows.is_empty() {
            let length: f64 = rows.iter().filter_map(|row| number_in(row, "length")).sum();
            facts.push(format!("length {}", metres(length)));
        }
        if !facts.is_empty() {
            out.push(format!("  {}", facts.join(" · ")));
        }
        let types = types_of(&rows);
        if !types.is_empty() {
            out.push(format!("  types: {}", types.join(", ")));
        }
        if rest > 0 {
            out.push(format!("  … and {rest} more"));
        }
    }

    // Doors and windows: their marks, their types and their sizes.
    for (category, word) in [("door", "DOORS"), ("window", "WINDOWS")] {
        if let Some((count, rows)) = element_rows(category, limit) {
            out.push(String::new());
            out.push(format!("{word} ({count})"));
            if rows.is_empty() {
                out.push("  none".to_string());
            }
            for row in &rows {
                let mark = text_in(row, "mark");
                let mark = if mark.is_empty() { "-" } else { mark };
                let width = number_in(row, "width").map_or_else(|| "?".to_string(), mm);
                let height = number_in(row, "height").map_or_else(|| "?".to_string(), mm);
                out.push(format!("  {:<8} {:<26} {:>7} × {:>7} mm", mark, text_in(row, "type"), width, height));
            }
            let rest = count.saturating_sub(rows.len() as u64);
            if rest > 0 {
                out.push(format!("  … and {rest} more"));
            }
        }
    }

    // Floors: how much ground they cover, and what they are of.
    if let Some((count, rows)) = element_rows("floor", limit) {
        out.push(String::new());
        out.push(format!("FLOORS ({count})"));
        if rows.is_empty() {
            out.push("  none".to_string());
        }
        let rest = count.saturating_sub(rows.len() as u64);
        let mut facts: Vec<String> = Vec::new();
        // The area is of the floors listed: only a whole list has a whole area.
        if rest == 0 && !rows.is_empty() {
            let area: f64 = rows.iter().filter_map(|row| number_in(row, "area_m2")).sum();
            facts.push(format!("area {}", square_metres(area)));
        }
        let types = types_of(&rows);
        if !types.is_empty() {
            facts.push(format!("types: {}", types.join(", ")));
        }
        if !facts.is_empty() {
            out.push(format!("  {}", facts.join(" · ")));
        }
        if rest > 0 {
            out.push(format!("  … and {rest} more"));
        }
    }

    // Roofs, of what they are.
    if let Some((count, rows)) = element_rows("roof", limit) {
        out.push(String::new());
        out.push(format!("ROOFS ({count})"));
        let types = types_of(&rows);
        if types.is_empty() {
            out.push("  none".to_string());
        } else {
            out.push(format!("  types: {}", types.join(", ")));
        }
    }

    // What the model's text says.
    if let Some((count, rows)) = element_rows("text", limit) {
        out.push(String::new());
        out.push(format!("TEXT NOTES ({count})"));
        if rows.is_empty() {
            out.push("  none".to_string());
        }
        for row in &rows {
            let text = text_in(row, "text").replace('\n', " / ");
            let short: String = text.chars().take(120).collect();
            out.push(format!("  - {short}"));
        }
        let rest = count.saturating_sub(rows.len() as u64);
        if rest > 0 {
            out.push(format!("  … and {rest} more"));
        }
    }

    // What the project has of its own: `brief` gives the library's size, not its contents.
    let mut library: Vec<String> = Vec::new();
    if let Some(counts) = model.get("library").and_then(Value::as_object) {
        for (key, word) in
            [("types", "types"), ("materials", "materials"), ("families", "families"), ("parameters", "project parameters")]
        {
            if let Some(n) = counts.get(key).and_then(Value::as_u64).filter(|n| *n > 0) {
                library.push(format!("{n} {word}"));
            }
        }
    }
    out.push(String::new());
    out.push(if library.is_empty() { "LIBRARY: nothing of its own yet".to_string() } else { format!("LIBRARY: {}", library.join(" · ")) });

    // What the model warns about, and how far its history goes.
    let warnings: Vec<String> = model
        .get("warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|w| w.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if warnings.is_empty() {
        out.push("WARNINGS: none".to_string());
    } else {
        out.push(format!("WARNINGS ({})", warnings.len()));
        for warning in warnings.iter().take(20) {
            out.push(format!("  - {warning}"));
        }
        if warnings.len() > 20 {
            out.push(format!("  … and {} more", warnings.len() - 20));
        }
    }
    let steps = model.get("history").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    out.push(format!("UNDO: {steps} step{}", if steps == 1 { "" } else { "s" }));

    let report = out.join("\n");
    info_log(&format!("the model report has {} lines", out.len()));
    Ok(json!({"report": report, "counts": counts}))
}

bimcraft_plugin_sdk::export! {
    "build" => build,
    "check" => check,
}