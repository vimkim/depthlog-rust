// logfmt2pretty.rs (colorized)
//
// Usage:
//   rustc logfmt2pretty.rs -O -o depthlog_pretty
//   ./depthlog_pretty < input.log
//
// Notes:
// - Uses ANSI colors when stdout is a TTY, or when FORCE_COLOR=1.
// - Disable colors with NO_COLOR=1.
// - Levels colorized: I,W,E,D,T (and fallback).
// - Function name colorized.

use std::collections::HashMap;
use std::io::{self, BufRead, IsTerminal};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let use_color = should_use_color(&stdout);

    let mut out = String::new();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { continue };
        let line = line.trim();
        if line.is_empty() { continue; }

        let fields = match parse_logfmt(line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let ts = fields.get("ts").map(|s| s.as_str()).unwrap_or("");
        let time = format_time_hms_millis(ts).unwrap_or_else(|| "??:??:??.???".to_string());

        let level = fields.get("level").map(|s| s.as_str()).unwrap_or("");
        let level_ch = map_level(level);

        let depth: usize = fields
            .get("depth")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let file = fields.get("file").map(|s| s.as_str()).unwrap_or("?");
        let line_no = fields.get("line").map(|s| s.as_str()).unwrap_or("?");
        let func = fields.get("func").map(|s| s.as_str()).unwrap_or("?");
        let msg = fields.get("msg").map(|s| s.as_str()).unwrap_or("");

        let indent = " ".repeat(depth.saturating_mul(4));

        let lvl = if use_color {
            color_level(level_ch)
        } else {
            level_ch.to_string()
        };

        let func_disp = if use_color {
            color_func(func)
        } else {
            func.to_string()
        };

        out.push_str(&format!(
            "{time} [{lvl}] {file}:{line_no} | {indent}{func_disp}: {msg}\n"
        ));
    }

    print!("{out}");
}

fn should_use_color(stdout: &io::Stdout) -> bool {
    // Respect NO_COLOR (https://no-color.org/)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    // Allow forcing
    if let Ok(v) = std::env::var("FORCE_COLOR") {
        if v != "0" && !v.is_empty() {
            return true;
        }
    }
    stdout.is_terminal()
}

fn map_level(level: &str) -> char {
    match level {
        "info" => 'I',
        "warn" | "warning" => 'W',
        "error" => 'E',
        "debug" => 'D',
        "trace" => 'T',
        other if !other.is_empty() => other.chars().next().unwrap_or('?').to_ascii_uppercase(),
        _ => '?',
    }
}

// ---------- ANSI coloring helpers ----------

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

// Standard ANSI colors
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";

fn color_level(ch: char) -> String {
    match ch {
        'E' => format!("{BOLD}{RED}{ch}{RESET}"),
        'W' => format!("{BOLD}{YELLOW}{ch}{RESET}"),
        'I' => format!("{BOLD}{GREEN}{ch}{RESET}"),
        'D' => format!("{BOLD}{BLUE}{ch}{RESET}"),
        'T' => format!("{BOLD}{MAGENTA}{ch}{RESET}"),
        _ => format!("{BOLD}{WHITE}{ch}{RESET}"),
    }
}

fn color_func(func: &str) -> String {
    // Function name: bold cyan (adjust if desired)
    format!("{BOLD}{CYAN}{func}{RESET}")
}

// ---------- logfmt parsing ----------

fn parse_logfmt(input: &str) -> Result<HashMap<String, String>, ()> {
    let mut m = HashMap::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(());
        }
        let key = &input[key_start..i];
        i += 1;

        let val = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let mut v = String::new();
            while i < bytes.len() {
                let c = bytes[i] as char;
                if c == '"' {
                    i += 1;
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

fn format_time_hms_millis(ts: &str) -> Option<String> {
    let t_pos = ts.find('T')?;
    let rest = &ts[t_pos + 1..];

    let end = rest
        .find('Z')
        .or_else(|| rest.find('+'))
        .or_else(|| rest.rfind('-'))?;

    let time_part = &rest[..end];
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

