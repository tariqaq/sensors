//! The CSV logs: one folder per sensor (`<n> - <name>`), one file per local day.
//!
//! ```text
//! time,sensor,name,temp_c,hum_pct,state
//! 2026-11-18 14:03:27,1,Living room,23.4,45.2,ok
//! 2026-11-18 14:03:41,1,Living room,,,missing
//! ```
//!
//! Renaming a sensor starts a new folder; the number in front keeps its history together (the
//! report and today's range read every folder of that number).

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use fenix_sdk::SensorState;

const HEADER: &str = "time,sensor,name,temp_c,hum_pct,state";

pub struct Logs {
    root: PathBuf,
}

/// One logged row, as read back.
pub struct Row {
    pub time: NaiveDateTime,
    pub temp: Option<f32>,
    pub hum: Option<f32>,
    pub state: String,
}

impl Logs {
    /// `%LOCALAPPDATA%\fenixOS-sensors\logs`, made if missing.
    pub fn open() -> io::Result<Logs> {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA not set"))?;
        let root = base.join("fenixOS-sensors").join("logs");
        fs::create_dir_all(&root)?;
        Ok(Logs { root })
    }

    /// Next to the exe, when %LOCALAPPDATA% cannot be used.
    /// Logs in a given folder (tests).
    #[cfg(test)]
    pub fn at(root: PathBuf) -> Logs {
        let _ = fs::create_dir_all(&root);
        Logs { root }
    }

    pub fn fallback() -> Logs {
        let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
        let root = dir.unwrap_or_else(|| PathBuf::from(".")).join("logs");
        let _ = fs::create_dir_all(&root);
        Logs { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The report's file, beside the logs folder.
    pub fn report_path(&self) -> PathBuf {
        self.root.parent().unwrap_or(&self.root).join("report.html")
    }

    fn folder(&self, n: usize, name: &str) -> PathBuf {
        self.root.join(format!("{n} - {}", folder_name(name, n)))
    }

    /// Every folder of sensor `n`, whatever it was called.
    fn folders(&self, n: usize) -> Vec<PathBuf> {
        let prefix = format!("{n} - ");
        let Ok(rd) = fs::read_dir(&self.root) else { return Vec::new() };
        rd.flatten()
            .filter(|e| e.path().is_dir() && e.file_name().to_string_lossy().starts_with(&prefix))
            .map(|e| e.path())
            .collect()
    }

    /// Append one reading; `at` is Unix ms.
    pub fn append(
        &self,
        n: usize,
        name: &str,
        at: u64,
        temp: Option<f32>,
        hum: Option<f32>,
        state: SensorState,
    ) -> io::Result<()> {
        let time = local(at);
        let dir = self.folder(n, name);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.csv", time.format("%Y-%m-%d")));
        let new = !path.exists();
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        if new {
            writeln!(f, "{HEADER}")?;
        }
        let ok = state == SensorState::Ok;
        let num = |v: Option<f32>| if ok { v.map(|x| format!("{x:.1}")).unwrap_or_default() } else { String::new() };
        writeln!(
            f,
            "{},{n},{},{},{},{}",
            time.format("%Y-%m-%d %H:%M:%S"),
            csv_field(name),
            num(temp),
            num(hum),
            state_name(state)
        )
    }

    /// Rows of sensor `n` on `day`, from all its folders, in time order.
    pub fn rows(&self, n: usize, day: NaiveDate) -> Vec<Row> {
        let file = format!("{}.csv", day.format("%Y-%m-%d"));
        let mut rows: Vec<Row> = Vec::new();
        for dir in self.folders(n) {
            let Ok(f) = File::open(dir.join(&file)) else { continue };
            for line in BufReader::new(f).lines().map_while(Result::ok).skip(1) {
                if let Some(r) = parse_row(&line) {
                    rows.push(r);
                }
            }
        }
        rows.sort_by_key(|r| r.time);
        rows
    }

    /// Today's lowest and highest values of sensor `n`, from its logs.
    pub fn today(&self, n: usize, day: NaiveDate) -> Day {
        let mut d = Day::new(day);
        for r in self.rows(n, day) {
            if let (Some(t), Some(h)) = (r.temp, r.hum) {
                d.add(t, h);
            }
        }
        d
    }

    /// Delete day files older than `keep` days (0 keeps all), and folders left empty.
    pub fn prune(&self, keep: u32, today: NaiveDate) {
        if keep == 0 {
            return;
        }
        let oldest = today - chrono::Duration::days(i64::from(keep) - 1);
        let Ok(rd) = fs::read_dir(&self.root) else { return };
        for dir in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            if let Ok(files) = fs::read_dir(&dir) {
                for f in files.flatten() {
                    let name = f.file_name().to_string_lossy().to_string();
                    let day = name.strip_suffix(".csv").and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
                    if day.is_some_and(|d| d < oldest) {
                        let _ = fs::remove_file(f.path());
                    }
                }
            }
            let _ = fs::remove_dir(&dir); // only succeeds when empty
        }
    }
}

/// Today's lowest and highest temperature and humidity of one sensor.
pub struct Day {
    t: Option<(f32, f32)>,
    h: Option<(f32, f32)>,
}

impl Day {
    pub fn new(_date: NaiveDate) -> Day {
        Day { t: None, h: None }
    }

    /// Take a good reading; true when the shown text changes.
    pub fn add(&mut self, t: f32, h: f32) -> bool {
        let before = self.text();
        let widen = |r: Option<(f32, f32)>, v: f32| Some(r.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        self.t = widen(self.t, t);
        self.h = widen(self.h, h);
        self.text() != before
    }

    /// Board text for the temperature and humidity ranges: "Today 21.2–24.8", "Today 40–55".
    pub fn text(&self) -> (String, String) {
        let t = self.t.map_or("Today --".into(), |(lo, hi)| format!("Today {lo:.1}–{hi:.1}"));
        let h = self.h.map_or("Today --".into(), |(lo, hi)| format!("Today {lo:.0}–{hi:.0}"));
        (t, h)
    }
}

pub fn local(at_ms: u64) -> NaiveDateTime {
    let utc = DateTime::from_timestamp_millis(at_ms as i64).unwrap_or_default();
    utc.with_timezone(&Local).naive_local()
}

pub fn unix(t: NaiveDateTime) -> i64 {
    Local.from_local_datetime(&t).earliest().map_or(0, |d| d.timestamp())
}

fn state_name(s: SensorState) -> &'static str {
    match s {
        SensorState::Ok => "ok",
        SensorState::Missing => "missing",
        SensorState::Error => "error",
    }
}

/// A folder name Windows takes: letters, digits, space and `-_.()`, no trailing dot or space.
fn folder_name(name: &str, n: usize) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { '_' })
        .take(32)
        .collect();
    let s = s.trim_end_matches(['.', ' ']).trim().to_string();
    if s.is_empty() { format!("Sensor {n}") } else { s }
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\"").replace(['\n', '\r'], " "))
    } else {
        s.to_string()
    }
}

/// Split one CSV line, with quoted fields.
fn split_csv(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                out.last_mut().unwrap().push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(String::new()),
            _ => out.last_mut().unwrap().push(c),
        }
    }
    out
}

fn parse_row(line: &str) -> Option<Row> {
    let f = split_csv(line);
    if f.len() < 6 {
        return None;
    }
    let time = NaiveDateTime::parse_from_str(&f[0], "%Y-%m-%d %H:%M:%S").ok()?;
    Some(Row { time, temp: f[3].parse().ok(), hum: f[4].parse().ok(), state: f[5].trim().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_round_trip() {
        let line = format!("2026-11-18 14:03:27,1,{},23.4,45.2,ok", csv_field("Room, \"north\""));
        let f = split_csv(&line);
        assert_eq!(f[2], "Room, \"north\"");
        let r = parse_row(&line).unwrap();
        assert_eq!(r.temp, Some(23.4));
        assert_eq!(r.state, "ok");
        assert!(parse_row("2026-11-18 14:03:41,1,Room,,,missing").unwrap().temp.is_none());
    }

    #[test]
    fn day_text() {
        let mut d = Day::new(NaiveDate::from_ymd_opt(2026, 11, 18).unwrap());
        assert_eq!(d.text().0, "Today --");
        assert!(d.add(23.4, 45.2));
        assert!(d.add(21.0, 55.0));
        assert!(!d.add(22.0, 50.0));
        assert_eq!(d.text(), ("Today 21.0–23.4".into(), "Today 45–55".into()));
    }

    #[test]
    fn folder_names() {
        assert_eq!(folder_name("Living room", 1), "Living room");
        assert_eq!(folder_name("a/b:c?", 1), "a_b_c_");
        assert_eq!(folder_name(" ... ", 2), "Sensor 2");
    }
}
