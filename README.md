# Sensors

A **fenixOS** app (`tariqaq/fenixOS`) for the ESP32-2432S028R board (the "CYD"). It shows live temperature and humidity from two DHT22 sensors wired to the board, serves a live web page from the board to your network, and logs every reading on the PC while it is connected. Made for fenixOS **v0.1.31**, which it also needs on the board (`min_fenixos`).

## What it does

- **On the board:** two cards, one per sensor, showing temperature and humidity with bars, the sensor state (Reading, Live, Not connected, Read error), the time of the last good read (`14:03:27`, from the board's clock) and today's low–high range. The board reads each sensor every 2 s, the fastest a DHT22 allows; the two are read 1 s apart. Landscape shows the cards side by side, portrait stacks them.
- **Without the PC:** the readings stay live. The board reads the sensors itself and shows "PC not connected" above them. Names and today's ranges keep their last values. Readings taken while the PC is away are not logged.
- **Switched off in the PC app:** the board shows "Turned off on host PC", as for any fenixOS app.
- **Web page:** in Wi-Fi mode with an SD card in, any phone or computer on the same network can open `http://fenixos.local/apps/io.github.tariqaq.sensors/` (also linked from the board's page at `http://fenixos.local/`). It shows the live readings with their read time, a 3D model of the board and sensors that turns slowly and pulses on each reading, and the last 24 hours as charts. The board serves the live readings itself, so the page keeps working while the PC is away; the names and the history come from the PC (history every 5 minutes). The 3D model loads three.js from a CDN, so it needs internet on the viewing device; without it the rest of the page still works.
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
| Web page on the board (Wi-Fi) | On by default. Off takes the page off the board. The page is public on your network: anyone on it can see the names, readings and history |
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
cargo install --git https://github.com/tariqaq/fenixOS --tag v0.1.31 fenix-pkg
pwsh tools/build.ps1        # dist/app.fxp, dist/app.exe; checks the manifest
python tools/app.py local   # dist/registry for FENIX_REGISTRY (docs/fenixos/registry.md)
cargo test -p app           # log format, ranges, report escaping, page file sizes
```

Without sensors wired, fake them with the `fenix` CLI (PC app quit): `fenix sensors fake 22 23.4 45.2`, `fenix sensors fake 27 missing`, then `fenix sensors fake 22 off`.

Release: bump `version` in `manifest.json` and `app/board/app.toml`, tag `v<version>` and push the tag. The workflow attaches the files to the release and uploads the filled manifest. Add it to `tariqaq/fenixOS` as `registry/apps/io.github.tariqaq.sensors.json` by PR (steps in `docs/fenixos/registry.md`).

## Layout

| Path | What |
|---|---|
| `AGENTS.md`, `CLAUDE.md` | Rules for AI agents |
| `docs/fenixos/` | fenixOS developer docs (copies from the v0.1.31 template) |
| `docs/requests/` | Feature requests sent to fenixOS: sensor support (done in v0.1.29 and v0.1.30), sensor time and web pages (done in v0.1.31) |
| `design/tokens.json` | Design tokens (copy) |
| `manifest.json` | Registry manifest: name, colour, permissions, settings |
| `app/board/` | Board half: `app.toml` (with the two `[[sensor]]` blocks), `layout.json`, `icon.png` |
| `app/pc/` | PC half (Rust, `fenix-sdk`): `main.rs`, `logs.rs` (CSV), `report.rs` and `report.html` (the report), `web.rs` and `web/` (the board's web page; serve `web/` locally and open `index.html?demo` to preview it with made-up data) |
| `tools/` | `build.ps1`, `app.py` |
| `.github/workflows/release.yml` | Tag to release |
