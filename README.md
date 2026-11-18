# Sensors

A **fenixOS** app (`tariqaq/fenixOS`) for the ESP32-2432S028R board (the "CYD"). It shows live temperature and humidity from two DHT22 sensors wired to the board. While the PC is connected, it logs every reading on the PC. Made for fenixOS **v0.1.30**; needs **v0.1.29** or later on the board (`min_fenixos`).

## What it does

- **On the board:** two cards, one per sensor, showing temperature and humidity with bars, the sensor state (Reading, Live, Not connected, Read error) and today's low–high range. The board reads each sensor every 2 s, the fastest a DHT22 allows; the two are read 1 s apart. Landscape shows the cards side by side, portrait stacks them.
- **Without the PC:** the readings stay live. The board reads the sensors itself and shows "PC not connected" above them. Names and today's ranges keep their last values. Readings taken while the PC is away are not logged.
- **Switched off in the PC app:** the board shows "Turned off on host PC", as for any fenixOS app.
- **On the PC:** every reading goes to a CSV file, and the app's page in the fenixOS PC app has the settings below.

## Wiring

Plug both sensors into **CN1**, the 4-pin JST 1.25 mm socket on the underside: `GND`, `IO22`, `IO27`, `3.3V`.

| Sensor | Data pin | Binding prefix |
|---|---|---|
| Sensor 1 | IO22 | `s1` |
| Sensor 2 | IO27 | `s2` |

Power both from `3.3V` and `GND`. Use one sensor per pin; DHT22 can't share a data wire. A 10 kΩ pull-up from data to 3.3 V helps on long wires: many DHT22 modules have one, and IO27 has one on the board.

## Settings (on the app's page in the PC app)

| Setting | What it does |
|---|---|
| Sensor 1 name, Sensor 2 name | Shown on the board, in the logs and in the report |
| Log a reading every | 2 s (every reading), 10 s, 1 min or 5 min. A change of state is always logged |
| Keep logs for | 30 days, 90 days, 1 year or forever |
| Open report | Builds an HTML report of the last 24 hours and 7 days and opens it in the browser |
| Open logs folder | Opens the logs folder in Explorer |

## Logs

`%LOCALAPPDATA%\fenixOS-sensors\logs\<n> - <name>\YYYY-MM-DD.csv`, one folder per sensor and one file per day:

```text
time,sensor,name,temp_c,hum_pct,state
2026-11-18 14:03:27,1,Living room,23.4,45.2,ok
2026-11-18 14:03:41,1,Living room,,,missing
```

Renaming a sensor starts a new folder. The number in front keeps its history together: the report and today's range read every folder with that number. The report is written to `%LOCALAPPDATA%\fenixOS-sensors\report.html`.

## Build, test, release

Tools: Rust (stable), Python 3.11+, and `fenix-pkg`:

```powershell
cargo install --git https://github.com/tariqaq/fenixOS --tag v0.1.30 fenix-pkg
pwsh tools/build.ps1        # dist/app.fxp, dist/app.exe; checks the manifest
python tools/app.py local   # dist/registry for FENIX_REGISTRY (docs/fenixos/registry.md)
cargo test -p app           # log format, ranges, report escaping
```

Without sensors wired, fake them with the `fenix` CLI (PC app quit): `fenix sensors fake 22 23.4 45.2`, `fenix sensors fake 27 missing`, then `fenix sensors fake 22 off`.

Release: bump `version` in `manifest.json` and `app/board/app.toml`, tag `v<version>` and push the tag. The workflow attaches the files to the release and uploads the filled manifest. Add it to `tariqaq/fenixOS` as `registry/apps/io.github.tariqaq.sensors.json` by PR (steps in `docs/fenixos/registry.md`).

## Layout

| Path | What |
|---|---|
| `AGENTS.md`, `CLAUDE.md` | Rules for AI agents |
| `docs/fenixos/` | fenixOS developer docs (copies from the v0.1.30 template) |
| `docs/requests/` | Feature requests sent to fenixOS: sensor support (done in v0.1.29), sensor time and web pages (planned) |
| `design/tokens.json` | Design tokens (copy) |
| `manifest.json` | Registry manifest: name, colour, permissions, settings |
| `app/board/` | Board half: `app.toml` (with the two `[[sensor]]` blocks), `layout.json`, `icon.png` |
| `app/pc/` | PC half (Rust, `fenix-sdk`): `main.rs`, `logs.rs` (CSV), `report.rs` and `report.html` (the report) |
| `tools/` | `build.ps1`, `app.py` |
| `.github/workflows/release.yml` | Tag to release |
