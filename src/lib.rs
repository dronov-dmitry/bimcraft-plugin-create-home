//! A test BIMCraft plugin: builds a small house with ordinary commands.
//!
//! One call makes the four walls, the floor, a door, a window, a room, a roof and a note. Every step is a command, so the
//! whole call is one undo step and a failure in the middle takes all of it back. It reads the model with the same
//! `elements` command in `check`, which answers with a `report` the window opens on screen, and in `summary`, which
//! answers with a `panel` the window draws: a table, a note and a badge. The panels themselves are declared in
//! `plugin.json` (ABI 2); the plugin writes no interface code.

use bimcraft_plugin_sdk::{Value, execute, host_info, info_log};
#[cfg(feature = "self-update")]
use bimcraft_plugin_sdk::{apply_update, check_update};
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
    // An empty field of a panel sends "", which is not a name: fall back to the default.
    let room_name = params
        .get("room")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Living Room")
        .to_string();
    let no_roof = params.get("no_roof").and_then(Value::as_bool).unwrap_or(false);
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
    let roof = if no_roof { Value::Null } else { execute("roof", &json!({"overhang": 500.0, "slope": 30.0}))? };
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

/// A line of Markdown text: no newline may break a heading, a bullet or a table cell.
fn md_line(text: &str) -> String {
    text.replace('\r', " ").replace('\n', " ")
}

/// A table cell: its `|` is escaped, so it never makes another column.
fn md_cell(text: &str) -> String {
    md_line(text).replace('\\', "\\\\").replace('|', "\\|")
}

/// A GitHub table: the header, the rules (`:---`, `---:`, `:---:`) and the rows.
fn md_table(out: &mut String, header: &[&str], rules: &[&str], rows: &[Vec<String>]) {
    out.push('|');
    for cell in header {
        out.push(' ');
        out.push_str(cell);
        out.push_str(" |");
    }
    out.push('\n');
    out.push('|');
    for rule in rules {
        out.push_str(rule);
        out.push('|');
    }
    out.push('\n');
    for row in rows {
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(&md_cell(cell));
            out.push_str(" |");
        }
        out.push('\n');
    }
    out.push('\n');
}

/// `check`: the model's composition as a person reads it — its levels, its elements by category and by level, the rooms
/// with their areas, the walls, the openings, the floors, the roofs, the notes, the library, the warnings and the undo
/// history. A host that reports `markdown_reports` gets a Markdown report with tables; an older host gets the plain-text
/// report of [`check_plain`]. It only reads the model, so it makes no undo step.
fn check(_: &Value) -> Result<Value, String> {
    let model = execute("inspect", &json!({"brief": true}))?;
    let info = host_info().unwrap_or(Value::Null);
    if info.get("markdown_reports").and_then(Value::as_bool) != Some(true) {
        return check_plain(&Value::Null);
    }
    let mut out = String::new();

    let project = text_in(&model, "name");
    let title = if project.is_empty() { "Model report".to_string() } else { format!("Model report — {}", md_line(project)) };
    out.push_str(&format!("# {title}\n\n"));

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
    out.push_str(&format!("{}\n\n", md_line(&head.join(" · "))));
    if let Some(path) = model.get("path").and_then(Value::as_str).filter(|p| !p.is_empty()) {
        out.push_str(&format!("File: `{}`\n\n", path.replace('`', "'")));
    }

    let levels: Vec<Value> = model.get("levels").and_then(Value::as_array).cloned().unwrap_or_default();
    out.push_str(&format!("## Levels ({})\n\n", levels.len()));
    if levels.is_empty() {
        out.push_str("The model has no levels.\n\n");
    } else {
        let rows: Vec<Vec<String>> = levels
            .iter()
            .map(|level| vec![text_in(level, "name").to_string(), format!("{} mm", mm(number_in(level, "elevation").unwrap_or(0.0)))])
            .collect();
        md_table(&mut out, &["Name", "Elevation"], &[":---", "---:"], &rows);
    }

    let counts = model.get("counts").cloned().unwrap_or(Value::Null);
    let mut categories: Vec<(String, u64)> = counts
        .as_object()
        .map(|c| c.iter().map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0))).filter(|(_, n)| *n > 0).collect())
        .unwrap_or_default();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let total: u64 = categories.iter().map(|(_, n)| *n).sum();
    out.push_str("## Elements\n\n");
    if categories.is_empty() {
        out.push_str("The model has no elements.\n\n");
    } else {
        let mut rows: Vec<Vec<String>> =
            categories.iter().map(|(category, n)| vec![plural(&category_name(category)), n.to_string()]).collect();
        rows.push(vec!["**Total**".to_string(), format!("**{total}**")]);
        md_table(&mut out, &["Category", "Count"], &[":---", "---:"], &rows);
    }

    let limit: u64 = if total <= 40 { 60 } else { 8 };

    if !levels.is_empty() && !categories.is_empty() {
        out.push_str("## By level\n\n");
        let pairs = levels.len() * categories.len();
        if pairs > 40 {
            out.push_str(&format!(
                "{pairs} level and category pairs: too many to count one by one; **Elements** has the totals.\n\n"
            ));
        } else {
            for level in &levels {
                let name = text_in(level, "name");
                let elevation = mm(number_in(level, "elevation").unwrap_or(0.0));
                out.push_str(&format!("### {} ({} mm)\n\n", md_line(name), elevation));
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
                    out.push_str(if refused { "*Could not be counted.*\n\n" } else { "*Nothing.*\n\n" });
                } else {
                    for item in here {
                        out.push_str(&format!("- {}\n", md_line(&item)));
                    }
                    out.push('\n');
                }
            }
        }
    }

    if let Some((count, rows)) = element_rows("room", limit) {
        out.push_str(&format!("## Rooms ({count})\n\n"));
        if rows.is_empty() {
            out.push_str("None.\n\n");
        } else {
            let mut area = 0.0;
            let mut table: Vec<Vec<String>> = Vec::new();
            for row in &rows {
                let here = number_in(row, "area_m2").unwrap_or(0.0);
                area += here;
                let state = if row.get("enclosed").and_then(Value::as_bool) == Some(true) { "yes".to_string() } else { "**NOT enclosed**".to_string() };
                table.push(vec![text_in(row, "number").to_string(), text_in(row, "name").to_string(), format!("{here:.1} m2"), state]);
            }
            md_table(&mut out, &["Number", "Name", "Area", "Enclosed"], &[":---", ":---", "---:", ":---:"], &table);
            let rest = count.saturating_sub(rows.len() as u64);
            if rest > 0 {
                out.push_str(&format!("… and {rest} more rooms.\n\n"));
            } else {
                out.push_str(&format!("**Total area:** {}\n\n", square_metres(area)));
            }
        }
    }

    if let Some((count, rows)) = element_rows("wall", limit) {
        out.push_str(&format!("## Walls ({count})\n\n"));
        if rows.is_empty() {
            out.push_str("None.\n\n");
        } else {
            let mut heights: Vec<String> = Vec::new();
            for row in &rows {
                let height = mm(number_in(row, "height").unwrap_or(0.0));
                if !heights.contains(&height) {
                    heights.push(height);
                }
            }
            let rest = count.saturating_sub(rows.len() as u64);
            if !heights.is_empty() {
                out.push_str(&format!("- **Height:** {} mm\n", heights.join(" / ")));
            }
            if rest == 0 {
                let length: f64 = rows.iter().filter_map(|row| number_in(row, "length")).sum();
                out.push_str(&format!("- **Length:** {}\n", metres(length)));
            }
            let types = types_of(&rows);
            if !types.is_empty() {
                out.push_str(&format!("- **Types:** {}\n", md_line(&types.join(", "))));
            }
            if rest > 0 {
                out.push_str(&format!("- … and {rest} more walls\n"));
            }
            out.push('\n');
        }
    }

    for (category, word) in [("door", "Doors"), ("window", "Windows")] {
        if let Some((count, rows)) = element_rows(category, limit) {
            out.push_str(&format!("## {word} ({count})\n\n"));
            if rows.is_empty() {
                out.push_str("None.\n\n");
            } else {
                let mut table: Vec<Vec<String>> = Vec::new();
                for row in &rows {
                    let mark = text_in(row, "mark");
                    let width = number_in(row, "width").map_or_else(|| "?".to_string(), mm);
                    let height = number_in(row, "height").map_or_else(|| "?".to_string(), mm);
                    table.push(vec![
                        if mark.is_empty() { "-".to_string() } else { mark.to_string() },
                        text_in(row, "type").to_string(),
                        format!("{width} mm"),
                        format!("{height} mm"),
                    ]);
                }
                md_table(&mut out, &["Mark", "Type", "Width", "Height"], &[":---", ":---", "---:", "---:"], &table);
                let rest = count.saturating_sub(rows.len() as u64);
                if rest > 0 {
                    out.push_str(&format!("… and {rest} more.\n\n"));
                }
            }
        }
    }

    if let Some((count, rows)) = element_rows("floor", limit) {
        out.push_str(&format!("## Floors ({count})\n\n"));
        if rows.is_empty() {
            out.push_str("None.\n\n");
        } else {
            let rest = count.saturating_sub(rows.len() as u64);
            if rest == 0 {
                let area: f64 = rows.iter().filter_map(|row| number_in(row, "area_m2")).sum();
                out.push_str(&format!("- **Area:** {}\n", square_metres(area)));
            }
            let types = types_of(&rows);
            if !types.is_empty() {
                out.push_str(&format!("- **Types:** {}\n", md_line(&types.join(", "))));
            }
            if rest > 0 {
                out.push_str(&format!("- … and {rest} more floors\n"));
            }
            out.push('\n');
        }
    }

    if let Some((count, rows)) = element_rows("roof", limit) {
        out.push_str(&format!("## Roofs ({count})\n\n"));
        let types = types_of(&rows);
        if types.is_empty() {
            out.push_str("None.\n\n");
        } else {
            out.push_str(&format!("- **Types:** {}\n\n", md_line(&types.join(", "))));
        }
    }

    if let Some((count, rows)) = element_rows("text", limit) {
        out.push_str(&format!("## Text notes ({count})\n\n"));
        if rows.is_empty() {
            out.push_str("None.\n\n");
        } else {
            for row in &rows {
                let text = text_in(row, "text").replace('\n', " / ");
                let short: String = text.chars().take(120).collect();
                out.push_str(&format!("- {}\n", md_line(&short)));
            }
            let rest = count.saturating_sub(rows.len() as u64);
            if rest > 0 {
                out.push_str(&format!("- … and {rest} more\n"));
            }
            out.push('\n');
        }
    }

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
    out.push_str("## Library\n\n");
    out.push_str(&format!(
        "{}\n\n",
        if library.is_empty() { "Nothing of its own yet.".to_string() } else { md_line(&library.join(" · ")) }
    ));

    let warnings: Vec<String> = model
        .get("warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|w| w.as_str().map(String::from)).collect())
        .unwrap_or_default();
    out.push_str("## Warnings\n\n");
    if warnings.is_empty() {
        out.push_str("None.\n\n");
    } else {
        for warning in warnings.iter().take(20) {
            out.push_str(&format!("- {}\n", md_line(warning)));
        }
        if warnings.len() > 20 {
            out.push_str(&format!("- … and {} more\n", warnings.len() - 20));
        }
        out.push('\n');
    }

    let steps = model.get("history").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    out.push_str(&format!("**Undo:** {steps} step{}\n", if steps == 1 { "" } else { "s" }));

    info_log(&format!("the model report has {} lines of Markdown", out.lines().count()));
    Ok(json!({"report": {"text": out, "format": "markdown"}, "counts": counts}))
}

/// `check_plain`: the same report as one string of monospace text, for a host that doesn't report
/// `markdown_reports`: its window shows a string report only, and an object would open nothing.
fn check_plain(_: &Value) -> Result<Value, String> {
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

/// `summary`: what the plugin's panels show. It reads the model, counts the categories on the chosen level or lists
/// the rooms with their areas, and answers with `panel`: a table and a note. It only reads, so it makes no
/// undo step; the panels' values arrive as ordinary `params`.
fn summary(params: &Value) -> Result<Value, String> {
    let model = execute("inspect", &json!({"brief": true}))?;
    let counts = model.get("counts").cloned().unwrap_or(Value::Null);
    let mut categories: Vec<(String, u64)> = counts
        .as_object()
        .map(|c| c.iter().map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0))).filter(|(_, n)| *n > 0).collect())
        .unwrap_or_default();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let total: u64 = categories.iter().map(|(_, n)| *n).sum();

    let level = params.get("level").and_then(Value::as_str).map(str::trim).unwrap_or("");
    let mode = params.get("mode").and_then(Value::as_str).unwrap_or("categories");
    let wall_type = params.get("wall_type").and_then(Value::as_str).map(str::trim).unwrap_or("");

    // A big model gets a shorter list: the table is a page of it, the note has the whole count.
    let limit: u64 = if total <= 40 { 60 } else { 8 };
    let mut query = json!({"category": "room", "limit": limit});
    if !level.is_empty() {
        query["level"] = json!(level);
    }
    let rooms = execute("elements", &query).ok();
    let room_count = rooms.as_ref().and_then(|r| r.get("count")).and_then(Value::as_u64).unwrap_or(0);
    let room_rows: Vec<Value> =
        rooms.as_ref().and_then(|r| r.get("elements")).and_then(Value::as_array).cloned().unwrap_or_default();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut on_level = total;
    if mode == "rooms" {
        for row in &room_rows {
            let area = number_in(row, "area_m2").unwrap_or(0.0);
            rows.push(vec![json!(text_in(row, "number")), json!(text_in(row, "name")), json!(format!("{area:.1} m2"))]);
        }
    } else {
        let scope = if level.is_empty() { "All levels" } else { level };
        on_level = 0;
        for (category, whole) in categories.iter().take(20) {
            let n = if level.is_empty() { *whole } else { level_count(category, level).unwrap_or(0) };
            if n > 0 {
                on_level += n;
                rows.push(vec![json!(scope), json!(plural(&category_name(category))), json!(n)]);
            }
        }
    }

    let project = text_in(&model, "name");
    let title = if project.is_empty() { "the untitled project" } else { project };
    let categories_text = format!("{} categor{}", categories.len(), if categories.len() == 1 { "y" } else { "ies" });
    let mut note = format!("{title}: {total} elements in {categories_text}");
    if !level.is_empty() && mode != "rooms" {
        note.push_str(&format!(" · {level}: {on_level}"));
    }
    if mode == "rooms" {
        let rooms_word = format!("{room_count} room{}", if room_count == 1 { "" } else { "s" });
        if level.is_empty() {
            note.push_str(&format!(" · {rooms_word}"));
        } else {
            note.push_str(&format!(" · {level}: {rooms_word}"));
        }
        if room_count as usize == room_rows.len() && !room_rows.is_empty() {
            let area: f64 = room_rows.iter().filter_map(|row| number_in(row, "area_m2")).sum();
            note.push_str(&format!(", {}", square_metres(area)));
        }
    }
    if !wall_type.is_empty() {
        note.push_str(&format!(" · wall type {wall_type}"));
    }
    let warnings = model.get("warnings").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    let steps = model.get("history").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    note.push_str(&format!(" · {warnings} warnings · {steps} undo steps"));

    info_log(&format!("the panel summary has {} rows", rows.len()));
    Ok(json!({
        "panel": {
            "text": note,
            "table": {"rows": rows},
            "error": null
        }
    }))
}

/// `open_file`: reads a project file from anywhere on the disk into the session, through `open`.
///
/// This one reaches the disk, so it only works when the plugin is given the `files` permission; the sandbox
/// refuses `open` before it runs otherwise and the error says exactly that. `open` replaces the project, so the
/// call is not an undo step: the history starts again with the file's own.
fn open_file(params: &Value) -> Result<Value, String> {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or("`path` is required: the project file to open (.bimcraft or .ifc)")?;
    info_log(&format!("opening {path}"));
    let reply = execute("open", &json!({"path": path}))?;
    // What the file brought in, as the model counts it now.
    let model = execute("inspect", &json!({"brief": true})).unwrap_or(Value::Null);
    let counts = model.get("counts").cloned().unwrap_or(Value::Null);
    let total: u64 = counts.as_object().map(|c| c.values().filter_map(Value::as_u64).sum()).unwrap_or(0);
    let name = model.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    info_log(&format!("opened {path}: {total} elements"));
    Ok(json!({"report": format!("OPENED {path}\n  project: {name}\n  elements: {total}"), "path": path, "name": name, "counts": counts, "open": reply}))
}

/// `update`: what the plugin's own release channel announces, and installing it on request.
///
/// The plugin has no network and no disk: the host reads the feed declared in `plugin.json` and replaces the
/// folder. The `update` permission is the user's, and the host asks before the first install; with automatic
/// updating on, newer versions are installed without a call from here. The new module runs after a restart, so
/// this answers with what happened rather than with a reload.
///
/// This needs a host that links `bimcraft.update_check` and `bimcraft.update_apply`, which is why it is off by
/// default: a module that imports them is refused by a host that has only `execute`, `log` and `info`.
#[cfg(feature = "self-update")]
fn update(params: &Value) -> Result<Value, String> {
    let install = params.get("install").and_then(Value::as_bool).unwrap_or(false);
    let checked = check_update()?;
    let current = checked.get("current").and_then(Value::as_str).unwrap_or("").to_string();
    let available = checked.get("available").and_then(Value::as_str).unwrap_or(&current).to_string();
    let newer = available != current;
    info_log(&format!("the channel has {available}; this build is {current}"));

    let mut reply = json!({"current": current, "available": available, "upToDate": !newer, "channel": checked});
    if install && newer {
        // A refusal is the user's, and the host words it; it comes back as an `Err` here.
        reply["installed"] = apply_update()?;
        reply["restart"] = json!(true);
    }
    Ok(reply)
}

#[cfg(not(feature = "self-update"))]
bimcraft_plugin_sdk::export! {
    "build" => build,
    "check" => check,
    "summary" => summary,
    "open_file" => open_file,
}

#[cfg(feature = "self-update")]
bimcraft_plugin_sdk::export! {
    "build" => build,
    "check" => check,
    "summary" => summary,
    "open_file" => open_file,
    "update" => update,
}