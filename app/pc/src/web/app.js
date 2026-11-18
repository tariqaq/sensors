// Sensors web page, served by the fenixOS board at /apps/<id>/ (docs/fenixos/app-format.md "App web
// page"). Live readings from the board's /sensors.json every 2 s; names (config.json) and the last
// 24 hours (history.json) come from the PC half, which uploads them while it runs.
"use strict";

const APP_ID = (location.pathname.match(/\/apps\/([^/]+)/) || [])[1] || "io.github.tariqaq.sensors";
const DEMO = location.protocol === "file:" || new URLSearchParams(location.search).has("demo");
const POLL_MS = 2000; // the board reads each sensor every 2 s; fenixOS asks pages not to poll harder
const COLORS = ["var(--s1)", "var(--s2)"];
const STATE = { reading: "Reading", ok: "Live", missing: "Not connected", error: "Read error" };

const S = {
  names: ["Sensor 1", "Sensor 2"],
  readings: [null, null], // per sensor: {state, label, temp, hum, read, age, pin}
  board: null,            // {time, pc}
  online: false,
  history: null,
};
window.SENSORS = S;

const $ = (s) => document.querySelector(s);
const el = (tag, attrs = {}, text) => {
  const ns = ["svg", "path", "line", "text", "circle", "rect"].includes(tag);
  const e = ns ? document.createElementNS("http://www.w3.org/2000/svg", tag) : document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (text !== undefined) e.textContent = text;
  return e;
};
const fmt = (v, d) => (v === null || v === undefined ? "--" : Number(v).toFixed(d));
const hm = { hour: "2-digit", minute: "2-digit" };

// ---- data -------------------------------------------------------------------------------------

async function getJSON(url) {
  const r = await fetch(url, { cache: "no-store" });
  if (!r.ok) throw new Error(url + " " + r.status);
  return r.json();
}

async function pollSensors() {
  try {
    const j = DEMO ? demo.sensors() : await getJSON("/sensors.json");
    S.board = { time: j.time, pc: j.pc };
    S.online = true;
    const mine = [null, null];
    for (const s of j.sensors || []) {
      const app = (s.apps || []).find((a) => a.id === APP_ID);
      const i = app ? { s1: 0, s2: 1 }[app.bind] : { 22: 0, 27: 1 }[s.pin];
      if (i !== undefined && !mine[i]) mine[i] = s;
    }
    S.readings = mine;
  } catch (e) {
    S.online = false;
  }
  render();
  dispatchEvent(new CustomEvent("sensors"));
}

async function pollConfig() {
  try {
    const c = DEMO ? demo.config() : await getJSON("config.json");
    if (Array.isArray(c.names)) S.names = c.names.map((n, i) => String(n || `Sensor ${i + 1}`));
  } catch (e) { /* no PC half yet: default names */ }
  try {
    S.history = DEMO ? demo.history() : await getJSON("history.json");
  } catch (e) {
    S.history = null;
  }
  legends();
  charts();
  render();
}

// ---- cards ------------------------------------------------------------------------------------

function buildCards() {
  const box = $("#cards");
  box.replaceChildren();
  [0, 1].forEach((i) => {
    const c = el("article", { class: "card", id: `card-${i}` });
    c.innerHTML = `
      <div class="name"><span class="sw" style="background:${COLORS[i]}"></span><span class="n"></span>
        <span class="state"><span class="dot"></span><span class="st"></span></span></div>
      <div class="now">
        <div><div class="lbl">Temperature</div><div class="big t">--<span class="unit">°C</span></div><div class="bar"><i class="bt"></i></div></div>
        <div><div class="lbl">Humidity</div><div class="big h">--<span class="unit">%</span></div><div class="bar"><i class="bh"></i></div></div>
      </div>
      <div class="foot"><span class="upd">Updated --:--:--</span><span class="ago"></span></div>`;
    box.append(c);
  });
}

const last = [null, null];

function render() {
  const pill = $("#pill"), pt = $("#pill-text");
  if (!S.online) {
    pill.className = "pill warn";
    pt.textContent = "Board not reachable";
    $("#meta").textContent = "Retrying every 2 s";
  } else {
    pill.className = S.board.pc ? "pill ok" : "pill";
    pt.textContent = S.board.pc ? "PC connected" : "PC not connected";
    $("#meta").textContent = S.board.time ? `Board time ${S.board.time}` : "Board clock not set";
  }
  [0, 1].forEach((i) => {
    const c = $(`#card-${i}`), r = S.readings[i];
    if (!c) return;
    c.querySelector(".n").textContent = S.names[i];
    const st = c.querySelector(".state");
    const ok = !!r && r.state === "ok";
    st.className = "state " + (ok ? "ok" : r && r.state !== "reading" ? "bad" : "");
    st.querySelector(".st").textContent = !r ? (S.online ? "Not set up" : "--") : r.label || STATE[r.state] || r.state;
    const t = c.querySelector(".t"), h = c.querySelector(".h");
    t.firstChild.nodeValue = fmt(r && r.temp, 1);
    h.firstChild.nodeValue = fmt(r && r.hum, 1);
    t.classList.toggle("stale", !ok || !S.online);
    h.classList.toggle("stale", !ok || !S.online);
    c.querySelector(".bt").style.width = r && r.temp !== null ? clamp((r.temp + 10) / 60) * 100 + "%" : "0";
    c.querySelector(".bh").style.width = r && r.hum !== null ? clamp(r.hum / 100) * 100 + "%" : "0";
    const upd = c.querySelector(".upd");
    const text = r && r.read ? `Updated ${r.read}` : "Updated --:--:--";
    if (upd.textContent !== text) {
      upd.textContent = text;
      if (last[i] !== null) { upd.classList.remove("flash"); void upd.offsetWidth; upd.classList.add("flash"); }
      last[i] = text;
    }
    c.querySelector(".ago").textContent = r && r.age !== null && r.age !== undefined ? ago(r.age) : "";
  });
}

const clamp = (x) => Math.max(0, Math.min(1, x));
const ago = (s) => (s < 5 ? "just now" : s < 120 ? `${s} s ago` : s < 7200 ? `${Math.round(s / 60)} min ago` : `${Math.round(s / 3600)} h ago`);

function legends() {
  document.querySelectorAll(".legend").forEach((lg) => {
    lg.replaceChildren(...S.names.map((n, i) => {
      const sp = el("span");
      sp.append(el("span", { class: "sw", style: `background:${COLORS[i]}` }), document.createTextNode(n));
      return sp;
    }));
  });
}

// ---- charts (last 24 hours, 5-minute averages from the PC) ------------------------------------

function niceTicks(lo, hi, n) {
  const span = hi - lo || 1, raw = span / n, mag = Math.pow(10, Math.floor(Math.log10(raw)));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw);
  const out = [], end = Math.ceil(hi / step) * step;
  for (let v = Math.floor(lo / step) * step; v <= end + 1e-9; v += step) out.push(+v.toFixed(6));
  return out;
}

function chart(id, key, dec, unit) {
  const wrap = $("#" + id);
  wrap.replaceChildren();
  const H0 = S.history;
  const all = H0 ? H0.series.flatMap((s) => s[key]).filter((v) => v !== null) : [];
  if (!all.length) {
    wrap.append(el("div", { class: "empty" }, "No history yet. The PC sends it every 5 minutes while the app is on."));
    return;
  }
  const n = H0.series[0][key].length;
  const W = wrap.clientWidth, H = wrap.clientHeight, m = { l: 40, r: 16, t: 10, b: 26 };
  let lo = Math.min(...all), hi = Math.max(...all);
  const pad = Math.max((hi - lo) * 0.15, key === "t" ? 0.5 : 2);
  const ticks = niceTicks(lo - pad, hi + pad, 4);
  lo = ticks[0]; hi = ticks[ticks.length - 1];
  const x = (i) => m.l + (i * (W - m.l - m.r)) / Math.max(n - 1, 1);
  const y = (v) => m.t + ((hi - v) * (H - m.t - m.b)) / (hi - lo || 1);
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": `${key === "t" ? "Temperature" : "Humidity"}, last 24 hours` });
  for (const v of ticks) {
    svg.append(el("line", { x1: m.l, x2: W - m.r, y1: y(v), y2: y(v), stroke: "var(--separator)" }));
    svg.append(el("text", { x: m.l - 8, y: y(v) + 4, "text-anchor": "end" }, fmt(v, Number.isInteger(v) ? 0 : 1)));
  }
  const every = (W < 480 ? 6 : 3) * 3600;
  for (let i = 0; i < n; i++) {
    const s = H0.start + i * H0.step, local = s - new Date(s * 1000).getTimezoneOffset() * 60;
    if (((local % every) + every) % every >= H0.step) continue;
    svg.append(el("text", { x: x(i), y: H - 6, "text-anchor": "middle" }, new Date(s * 1000).toLocaleTimeString(undefined, hm)));
  }
  H0.series.forEach((s, si) => {
    let d = "", pen = false;
    s[key].forEach((v, i) => {
      if (v === null) { pen = false; return; }
      d += (pen ? "L" : "M") + x(i).toFixed(1) + " " + y(v).toFixed(1);
      pen = true;
    });
    if (d) svg.append(el("path", { d, fill: "none", stroke: COLORS[si], "stroke-width": 2, "stroke-linejoin": "round", "stroke-linecap": "round" }));
  });
  const cross = el("line", { y1: m.t, y2: H - m.b, stroke: "var(--label-2)", opacity: 0 });
  const dots = H0.series.map((_, si) => el("circle", { r: 4, fill: COLORS[si], stroke: "var(--surface-1)", "stroke-width": 2, opacity: 0 }));
  const hit = el("rect", { x: m.l, y: m.t, width: Math.max(W - m.l - m.r, 1), height: Math.max(H - m.t - m.b, 1), fill: "transparent" });
  svg.append(cross, ...dots, hit);
  wrap.append(svg);
  const tip = el("div", { class: "tip" });
  wrap.append(tip);
  hit.addEventListener("pointerleave", () => { cross.setAttribute("opacity", 0); dots.forEach((d) => d.setAttribute("opacity", 0)); tip.style.opacity = 0; });
  hit.addEventListener("pointermove", (ev) => {
    const r = svg.getBoundingClientRect();
    const i = Math.max(0, Math.min(n - 1, Math.round((((ev.clientX - r.left) * W) / r.width - m.l) * (n - 1) / (W - m.l - m.r))));
    cross.setAttribute("x1", x(i)); cross.setAttribute("x2", x(i)); cross.setAttribute("opacity", 0.5);
    tip.replaceChildren(el("div", { class: "t" }, new Date((H0.start + i * H0.step) * 1000).toLocaleTimeString(undefined, hm)));
    H0.series.forEach((s, si) => {
      const v = s[key][i];
      dots[si].setAttribute("opacity", v === null ? 0 : 1);
      if (v !== null) { dots[si].setAttribute("cx", x(i)); dots[si].setAttribute("cy", y(v)); }
      const row = el("div", { class: "row" });
      row.append(el("span", { class: "sw", style: `background:${COLORS[si]}` }), el("span", {}, `${S.names[si]}  ${fmt(v, dec)} ${unit}`));
      tip.append(row);
    });
    tip.style.opacity = 1;
    const tx = x(i) + 12 + tip.offsetWidth > W ? x(i) - 12 - tip.offsetWidth : x(i) + 12;
    tip.style.left = tx + "px"; tip.style.top = m.t + "px";
  });
}

function charts() {
  chart("plot-t", "t", 1, "°C");
  chart("plot-h", "h", 1, "%");
}

// ---- demo data (opened as a file, or with ?demo) ----------------------------------------------

const demo = {
  t0: Date.now(),
  sensors() {
    const k = (Date.now() - this.t0) / 1000, now = new Date();
    const read = now.toLocaleTimeString("en-GB");
    return { time: read, pc: true, sensors: [
      { pin: 22, kind: "dht22", state: "ok", label: "Live", temp: +(23.4 + Math.sin(k / 20)).toFixed(1), hum: +(45 + Math.cos(k / 30) * 3).toFixed(1), read, age: 0, apps: [{ id: APP_ID, bind: "s1" }] },
      { pin: 27, kind: "dht22", state: k % 40 > 30 ? "missing" : "ok", label: k % 40 > 30 ? "Not connected" : "Live", temp: +(8.2 + Math.sin(k / 15) * 2).toFixed(1), hum: +(68 + Math.sin(k / 25) * 5).toFixed(1), read, age: 1, apps: [{ id: APP_ID, bind: "s2" }] },
    ] };
  },
  config() { return { names: ["Living room", "Balcony"] }; },
  history() {
    const step = 300, n = 289, start = Math.floor(Date.now() / 1000 / step) * step - (n - 1) * step;
    const mk = (f) => Array.from({ length: n }, (_, i) => (i > 150 && i < 170 ? null : +f(((start + i * step) / 86400) * 2 * Math.PI).toFixed(1)));
    return { start, step, series: [
      { t: mk((p) => 22.5 + 2 * Math.sin(p)), h: mk((p) => 48 - 4 * Math.sin(p)) },
      { t: mk((p) => 8 + 5 * Math.sin(p - 0.6)), h: mk((p) => 70 + 8 * Math.cos(p)) },
    ] };
  },
};

// ---- start ------------------------------------------------------------------------------------

buildCards();
legends();
pollSensors();
pollConfig();
setInterval(pollSensors, POLL_MS);
setInterval(pollConfig, 60000);
let rt;
addEventListener("resize", () => { clearTimeout(rt); rt = setTimeout(charts, 120); });
