//! PC half of the Sensors app (docs/fenixos/pc-half.md). The board reads the two DHT22 sensors
//! itself and shows them; this half gets every reading (`Message::Sensor`) while the app is on and
//! the board is connected, and:
//! - logs them to `%LOCALAPPDATA%\fenixOS-sensors\logs\<n> - <name>\YYYY-MM-DD.csv`,
//! - sends the sensor names and today's lowest and highest values to the board,
//! - builds an HTML report and opens the logs folder (the two button settings),
//! - puts the web page on the board (fenixOS 0.1.31), served at `http://fenixos.local/apps/<id>/`.
#![windows_subsystem = "windows"]

mod logs;
mod report;
mod web;

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use chrono::Local;
use fenix_sdk::{App, Message, SensorState};

use logs::{Day, Logs};

/// One wired sensor: `n` is 1 (IO22, binding prefix `s1`) or 2 (IO27, `s2`).
struct Sensor {
    n: usize,
    name: String,
    /// Unix ms and state of the last logged row, to space rows by the "log every" setting.
    last: Option<(u64, SensorState)>,
    today: Day,
}

fn main() {
    let Ok(app) = App::connect() else {
        std::process::exit(2); // started by hand, not by fenixOS
    };
    let logs = match Logs::open() {
        Ok(l) => l,
        Err(e) => {
            app.log(fenix_sdk::Level::Error, format!("Sensors: no logs folder: {e}"));
            Logs::fallback()
        }
    };
    app.log_info(format!("Sensors v{} started, logs in {}", app.version, logs.root().display()));

    let mut date = Local::now().date_naive();
    let mut sensors: Vec<Sensor> = (1..=2)
        .map(|n| Sensor { n, name: name_setting(&app, n), last: None, today: logs.today(n, date) })
        .collect();
    logs.prune(keep_days(&app), date);
    let mut page = web::Web::new();

    loop {
        match app.next(Duration::from_secs(1)) {
            Some(Message::Stop) => break,
            // The board keeps no PC-set values while closed: send them all when it opens.
            Some(Message::Opened) => push(&app, &sensors),
            Some(Message::Settings(_)) => {
                for s in &mut sensors {
                    s.name = name_setting(&app, s.n);
                }
                logs.prune(keep_days(&app), date);
                push(&app, &sensors);
            }
            Some(Message::Sensor { bind, temp, hum, state, at, .. }) => {
                let Some(s) = sensors.iter_mut().find(|s| bind == format!("s{}", s.n)) else {
                    continue;
                };
                let ok = state == SensorState::Ok;
                if let (true, Some(t), Some(h)) = (ok, temp, hum) {
                    if s.today.add(t, h) {
                        push(&app, std::slice::from_ref(s));
                    }
                }
                let every = log_every(&app) * 1000;
                let due = match s.last {
                    None => true,
                    // 0.5 s of slack: readings come every 2 s, not to the millisecond.
                    Some((t0, st0)) => st0 != state || at + 500 >= t0 + every,
                };
                if due {
                    if let Err(e) = logs.append(s.n, &s.name, at, temp, hum, state) {
                        app.log(fenix_sdk::Level::Warn, format!("Sensors: log write failed: {e}"));
                    }
                    s.last = Some((at, state));
                }
            }
            Some(Message::Button { key }) => match key.as_str() {
                "open_logs" => open(logs.root()),
                "report" => {
                    let names = [sensors[0].name.clone(), sensors[1].name.clone()];
                    match report::write(&logs, &names) {
                        Ok(path) => open(&path),
                        Err(e) => app.log(fenix_sdk::Level::Warn, format!("Sensors: report failed: {e}")),
                    }
                }
                _ => {}
            },
            _ => {}
        }
        // A new day: today's lowest and highest start again, old logs go.
        let now = Local::now().date_naive();
        if now != date {
            date = now;
            for s in &mut sensors {
                s.today = Day::new(date);
            }
            logs.prune(keep_days(&app), date);
            push(&app, &sensors);
        }
        let names = [sensors[0].name.clone(), sensors[1].name.clone()];
        page.tick(&app, &logs, &names, web_on(&app));
    }
}

/// Names and today's ranges onto the board (only while the app's screen is open).
fn push(app: &App, sensors: &[Sensor]) {
    if !app.is_open() {
        return; // fenixOS drops values for a closed app
    }
    for s in sensors {
        app.set(&format!("name{}", s.n), fenix_sdk::text::fit(&s.name, false));
        let (t, h) = s.today.text();
        app.set(&format!("t{}_range", s.n), t);
        app.set(&format!("h{}_range", s.n), h);
    }
}

fn setting_str(app: &App, key: &str) -> Option<String> {
    app.settings().get(key).and_then(|v| v.as_str().map(|s| s.trim().to_string()))
}

fn name_setting(app: &App, n: usize) -> String {
    setting_str(app, &format!("name{n}")).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("Sensor {n}"))
}

/// The "log every" setting in seconds (a choice: "2", "10", "60" or "300").
fn log_every(app: &App) -> u64 {
    setting_str(app, "log_every").and_then(|v| v.parse().ok()).unwrap_or(2).max(2)
}

/// The "web" toggle: the page on the board, on unless switched off.
fn web_on(app: &App) -> bool {
    app.settings().get("web").and_then(|v| v.as_bool()).unwrap_or(true)
}

/// The "keep" setting in days; 0 keeps everything.
fn keep_days(app: &App) -> u32 {
    setting_str(app, "keep").and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Open a folder in Explorer, or a file in its default program (the report in the browser).
fn open(path: &Path) {
    let _ = Command::new("explorer").arg(path).spawn();
}

