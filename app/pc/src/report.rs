//! The HTML report: last 24 hours and last 7 days of both sensors, from the CSV logs. One
//! self-contained file (data inlined, no scripts from the network; Inter from Google Fonts when
//! online, the system font otherwise), in the fenixOS colours, opened in the default browser.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::PathBuf;

use chrono::{Duration, Local};

use crate::logs::{self, Logs, Row};

const PAGE: &str = include_str!("report.html");

/// One time range of the report: buckets of `step` seconds from `start` (Unix s).
struct Range {
    key: &'static str,
    start: i64,
    step: i64,
    count: usize,
}

pub fn write(logs: &Logs, names: &[String; 2]) -> io::Result<PathBuf> {
    let now = Local::now();
    let now_s = now.timestamp();
    let today = now.date_naive();
    // Rows of the last 8 calendar days covers 7 × 24 h whatever the time of day.
    let rows: Vec<Vec<Row>> = (1..=2)
        .map(|n| (0..8).rev().flat_map(|d| logs.rows(n, today - Duration::days(d))).collect())
        .collect();

    let ranges = [
        Range { key: "day", step: 300, start: 0, count: 0 },
        Range { key: "week", step: 1800, start: 0, count: 0 },
    ]
    .map(|mut r| {
        let span = if r.key == "day" { 86_400 } else { 7 * 86_400 };
        r.start = (now_s - span) / r.step * r.step;
        r.count = ((now_s - r.start) / r.step + 1) as usize;
        r
    });

    let mut j = String::from("{");
    let _ = write!(j, "\"made\":{now_s},\"folder\":{},", js_str(&logs.root().display().to_string()));
    j.push_str("\"sensors\":[");
    for (i, rs) in rows.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let _ = write!(j, "{{\"name\":{},\"last\":", js_str(&names[i]));
        match rs.last() {
            Some(r) => {
                let _ = write!(
                    j,
                    "{{\"at\":{},\"t\":{},\"h\":{},\"state\":{}}}",
                    logs::unix(r.time),
                    num(r.temp),
                    num(r.hum),
                    js_str(&r.state)
                );
            }
            None => j.push_str("null"),
        }
        j.push('}');
    }
    j.push_str("],\"ranges\":{");
    for (k, r) in ranges.iter().enumerate() {
        if k > 0 {
            j.push(',');
        }
        let _ = write!(j, "\"{}\":{{\"start\":{},\"step\":{},\"series\":[", r.key, r.start, r.step);
        for (i, rs) in rows.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            let (t, h) = buckets(rs, r);
            let _ = write!(j, "{{\"t\":{},\"h\":{}}}", list(&t), list(&h));
        }
        j.push_str("]}");
    }
    j.push_str("}}");

    let path = logs.report_path();
    // `</` cannot appear inside the inlined JSON (names are escaped), so a script tag holds it.
    fs::write(&path, PAGE.replace("__DATA__", &j))?;
    Ok(path)
}

/// Average temperature and humidity per bucket; None where the bucket has no good reading.
fn buckets(rows: &[Row], r: &Range) -> (Vec<Option<f32>>, Vec<Option<f32>>) {
    let mut sum = vec![(0f64, 0f64, 0u32); r.count];
    for row in rows {
        let (Some(t), Some(h)) = (row.temp, row.hum) else { continue };
        let at = logs::unix(row.time);
        if at < r.start {
            continue;
        }
        let i = ((at - r.start) / r.step) as usize;
        if let Some(b) = sum.get_mut(i) {
            b.0 += f64::from(t);
            b.1 += f64::from(h);
            b.2 += 1;
        }
    }
    let avg = |s: f64, c: u32| if c == 0 { None } else { Some((s / f64::from(c)) as f32) };
    sum.iter().map(|&(t, h, c)| (avg(t, c), avg(h, c))).unzip()
}

fn num(v: Option<f32>) -> String {
    v.map_or("null".into(), |x| format!("{x:.1}"))
}

fn list(v: &[Option<f32>]) -> String {
    let items: Vec<String> = v.iter().map(|x| num(*x)).collect();
    format!("[{}]", items.join(","))
}

/// A JSON string that is also safe inside a `<script>` element.
fn js_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '<' => o.push_str("\\u003c"),
            '>' => o.push_str("\\u003e"),
            '&' => o.push_str("\\u0026"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a week of made-up readings and the report, into REPORT_DEMO_DIR when it is set.
    #[test]
    fn demo_report() {
        let Some(dir) = std::env::var_os("REPORT_DEMO_DIR") else { return };
        let logs = Logs::at(PathBuf::from(dir).join("logs"));
        let now = Local::now().timestamp();
        for k in 0..(7 * 24 * 60) {
            let at = now - k * 60;
            let ph = (at as f64 / 86_400.0) * std::f64::consts::TAU;
            let gap = k > 600 && k < 760;
            let t1 = 22.5 + 2.0 * ph.sin();
            let t2 = 8.0 + 5.0 * (ph - 0.6).sin();
            let st = if gap { fenix_sdk::SensorState::Missing } else { fenix_sdk::SensorState::Ok };
            let _ = logs.append(1, "Living room", (at * 1000) as u64, Some(t1 as f32), Some((48.0 - 4.0 * ph.sin()) as f32), fenix_sdk::SensorState::Ok);
            let _ = logs.append(2, "Balcony", (at * 1000) as u64, Some(t2 as f32), Some((70.0 + 8.0 * ph.cos()) as f32), st);
        }
        let p = write(&logs, &["Living room".into(), "Balcony".into()]).unwrap();
        assert!(p.exists());
    }

    #[test]
    fn js_strings_are_script_safe() {
        assert_eq!(js_str("a\"b</script>"), "\"a\\\"b\\u003c/script\\u003e\"");
    }
}
