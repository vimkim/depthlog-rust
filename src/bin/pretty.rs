// logfmt2pretty.rs
//
// Usage:
//   cargo run --release < input.log
// or:
//   rustc logfmt2pretty.rs -O && ./logfmt2pretty < input.log
//
// Output example:
//   16:13:50.091 [I] locator_sr.c:7252 |     xlocator_force: recdes length: 72, area_size: 88

use std::collections::HashMap;
use std::io::{self, BufRead};

fn main() {
    let stdin = io::stdin();
    let mut out = String::new();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { continue };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields = match parse_logfmt(line) {
            Ok(m) => m,
            Err(_) => continue, // or: eprintln!("failed to parse: {line}");
        };

        let ts = fields.get("ts").map(|s| s.as_str()).unwrap_or("");
        let time = format_time_hms_millis(ts).unwrap_or_else(|| "??:??:??.???".to_string());

        let level = fields.get("level").map(|s| s.as_str()).unwrap_or("");
        let level_ch = match level {
            "info" => 'I',
            "warn" | "warning" => 'W',
            "error" => 'E',
            "debug" => 'D',
            "trace" => 'T',
            other if !other.is_empty() => other.chars().next().unwrap_or('?').to_ascii_uppercase(),
            _ => '?',
        };

        let depth: usize = fields
            .get("depth")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let file = fields.get("file").map(|s| s.as_str()).unwrap_or("?");
        let line_no = fields.get("line").map(|s| s.as_str()).unwrap_or("?");
        let func = fields.get("func").map(|s| s.as_str()).unwrap_or("?");
        let msg = fields.get("msg").map(|s| s.as_str()).unwrap_or("");

        // Indentation: 4 spaces per depth level (tweak as desired).
        let indent = " ".repeat(depth.saturating_mul(4));

        out.push_str(&format!(
            "{time} [{level_ch}] {file}:{line_no} | {indent}{func}: {msg}\n"
        ));
    }

    print!("{out}");
}

/// Minimal logfmt parser:
/// - key=value pairs separated by spaces
/// - values may be quoted with "..."
/// - supports escapes in quoted strings: \", \\ , \n , \t , \r
fn parse_logfmt(input: &str) -> Result<HashMap<String, String>, ()> {
    let mut m = HashMap::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // skip spaces
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // parse key
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(());
        }
        let key = &input[key_start..i];
        i += 1; // skip '='

        // parse value
        let val = if i < bytes.len() && bytes[i] == b'"' {
            i += 1; // skip opening quote
            let mut v = String::new();
            while i < bytes.len() {
                let c = bytes[i] as char;
                if c == '"' {
                    i += 1; // closing quote
                    break;
                }
                if c == '\\' {
                    i += 1;
                    if i >= bytes.len() {
                        break;
                    }
                    let esc = bytes[i] as char;
                    match esc {
                        '"' => v.push('"'),
                        '\\' => v.push('\\'),
                        'n' => v.push('\n'),
                        't' => v.push('\t'),
                        'r' => v.push('\r'),
                        _ => v.push(esc),
                    }
                    i += 1;
                } else {
                    v.push(c);
                    i += 1;
                }
            }
            v
        } else {
            let val_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            input[val_start..i].to_string()
        };

        m.insert(key.to_string(), val);
    }

    Ok(m)
}

/// Parses RFC3339 timestamps like "2026-01-16T16:13:50.091+09:00"
/// and returns "16:13:50.091".
fn format_time_hms_millis(ts: &str) -> Option<String> {
    // Avoid extra deps; chrono is commonly used but not required if you compile with rustc only.
    // We'll do a light parse: find 'T', then take up to timezone sign.
    // Expected: YYYY-MM-DDTHH:MM:SS(.mmm)?(+|-)HH:MM or Z
    let t_pos = ts.find('T')?;
    let rest = &ts[t_pos + 1..];

    // end at 'Z' or '+' or '-'
    let end = rest
        .find('Z')
        .or_else(|| rest.find('+'))
        .or_else(|| rest.rfind('-'))?; // last '-' avoids the date dashes

    let time_part = &rest[..end]; // "HH:MM:SS.091" or "HH:MM:SS"
    // Normalize to milliseconds if missing
    if let Some(dot) = time_part.find('.') {
        let (hms, frac) = time_part.split_at(dot);
        let frac = &frac[1..];
        let ms = match frac.len() {
            0 => "000".to_string(),
            1 => format!("{frac}00"),
            2 => format!("{frac}0"),
            _ => frac[..3].to_string(),
        };
        Some(format!("{hms}.{ms}"))
    } else {
        Some(format!("{time_part}.000"))
    }
}
