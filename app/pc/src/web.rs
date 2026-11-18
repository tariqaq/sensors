//! The app's web page on the board (fenixOS 0.1.31, `docs/fenixos/app-format.md` "App web page").
//! The PC half puts the page under `web/` in the app's folder on the board's SD card, and the board
//! serves it at `http://fenixos.local/apps/<id>/` in Wi-Fi mode. The page reads live readings from
//! the board itself (`/sensors.json`), so it keeps working while the PC is away; this half adds
//! `config.json` (the sensor names) and `history.json` (the last 24 hours, every 5 minutes).
//!
//! Everything in `web/` is public on the network: only names and readings go there.

use std::time::{Duration, Instant};

use fenix_sdk::{App, Level};

use crate::logs::Logs;
use crate::report;

const FILES: [(&str, &str); 4] = [
    ("web/index.html", include_str!("web/index.html")),
    ("web/app.css", include_str!("web/app.css")),
    ("web/app.js", include_str!("web/app.js")),
    ("web/scene.js", include_str!("web/scene.js")),
];
const VERSION_FILE: &str = "web/version.txt";
const CONFIG_FILE: &str = "web/config.json";
const HISTORY_FILE: &str = "web/history.json";
const HISTORY_EVERY: Duration = Duration::from_secs(300);
const RETRY_AFTER: Duration = Duration::from_secs(120);
const MAX_FILE: usize = 32 * 1024; // put_file's limit

#[derive(Default)]
pub struct Web {
    connected: bool,
    page_ok: bool,
    names: Option<[String; 2]>,
    history_at: Option<Instant>,
    retry_at: Option<Instant>,
    cleared: bool,
}

impl Web {
    pub fn new() -> Web {
        Web::default()
    }

    /// Call often: uploads what is missing or stale while the board is connected. File calls
    /// block until the board answers, so this does at most one round of them per call.
    pub fn tick(&mut self, app: &App, logs: &Logs, names: &[String; 2], enabled: bool) {
        let connected = app.board_connected();
        if connected != self.connected {
            *self = Web { connected, ..Web::default() };
        }
        if !connected || self.retry_at.is_some_and(|t| Instant::now() < t) {
            return;
        }
        let r = if enabled { self.sync(app, logs, names) } else { self.clear(app) };
        if let Err(e) = r {
            // No card, USB-only firmware without a card, or an older fenixOS: try again later.
            app.log(Level::Warn, format!("Sensors: web page not updated on the board: {e}"));
            self.retry_at = Some(Instant::now() + RETRY_AFTER);
        }
    }

    fn sync(&mut self, app: &App, logs: &Logs, names: &[String; 2]) -> Result<(), String> {
        self.cleared = false;
        if !self.page_ok {
            let v = version();
            let have = app.get_file(VERSION_FILE).ok().map(|d| String::from_utf8_lossy(&d).trim().to_string());
            if have.as_deref() != Some(v.as_str()) {
                for (name, body) in FILES {
                    put(app, name, body)?;
                }
                put(app, VERSION_FILE, &v)?;
                app.log_info(format!("Sensors: web page {v} put on the board"));
            }
            self.page_ok = true;
        }
        if self.names.as_ref() != Some(names) {
            put(app, CONFIG_FILE, &config_json(names))?;
            self.names = Some(names.clone());
        }
        if self.history_at.is_none_or(|t| t.elapsed() >= HISTORY_EVERY) {
            put(app, HISTORY_FILE, &report::history_json(logs))?;
            self.history_at = Some(Instant::now());
        }
        Ok(())
    }

    /// The "web page" setting is off: take the page off the board (once per connection).
    fn clear(&mut self, app: &App) -> Result<(), String> {
        if !self.cleared {
            for name in FILES.iter().map(|f| f.0).chain([VERSION_FILE, CONFIG_FILE, HISTORY_FILE]) {
                let _ = app.delete_file(name); // a file that is not there is fine
            }
            *self = Web { connected: self.connected, cleared: true, ..Web::default() };
        }
        Ok(())
    }
}

fn put(app: &App, name: &str, body: &str) -> Result<(), String> {
    if body.len() > MAX_FILE {
        return Err(format!("{name} is {} B, over the 32 KB file limit", body.len()));
    }
    app.put_file(name, body.as_bytes()).map_err(|e| format!("{name}: {e}"))
}

fn config_json(names: &[String; 2]) -> String {
    format!("{{\"names\":[{},{}]}}", report::js_str(&names[0]), report::js_str(&names[1]))
}

/// A short hash of the page files (FNV-1a), so an unchanged page is not sent again.
fn version() -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, body) in FILES {
        for b in name.bytes().chain(body.bytes()) {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{:016x}", h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_files_fit_the_board() {
        for (name, body) in FILES {
            assert!(body.len() <= MAX_FILE, "{name} is {} B", body.len());
        }
        assert_eq!(config_json(&["A \"b\"".into(), "C".into()]), r#"{"names":["A \"b\"","C"]}"#);
    }
}
