// 3D model of the board and its two DHT22 sensors (three.js from a pinned CDN version). The board's
// screen shows the live readings, and each sensor's ring pulses in its colour on every new reading.
// Without WebGL or the CDN the stage stays hidden and the page works as before.
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { RoundedBoxGeometry } from "three/addons/geometries/RoundedBoxGeometry.js";

const S = window.SENSORS;
const stage = document.getElementById("stage");
const canvas = document.getElementById("scene");
const tags = [document.getElementById("tag-1"), document.getElementById("tag-2")];
const IDENT = [0x3987e5, 0xd95926];
const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;

function start() {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  stage.hidden = false;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 100);
  camera.position.set(5, 3.4, 22);
  const controls = new OrbitControls(camera, canvas);
  controls.target.set(0, -1.1, 1.2);
  controls.enableZoom = false;
  controls.enablePan = false;
  controls.enableDamping = true;
  controls.autoRotate = !reduce;
  controls.autoRotateSpeed = 0.7;
  controls.minPolarAngle = Math.PI * 0.25;
  controls.maxPolarAngle = Math.PI * 0.55;

  scene.add(new THREE.HemisphereLight(0xffffff, 0x1c1c1e, 1.6));
  const sun = new THREE.DirectionalLight(0xffffff, 1.4);
  sun.position.set(4, 8, 6);
  scene.add(sun);

  const mat = (color, o = {}) => new THREE.MeshStandardMaterial({ color, roughness: 0.6, metalness: 0, ...o });

  // ---- the board (ESP32-2432S028R in its acrylic case), standing up, screen to the front
  const board = new THREE.Group();
  board.rotation.x = -0.12;
  scene.add(board);
  board.add(new THREE.Mesh(new THREE.BoxGeometry(8.6, 5.0, 0.16), mat(0xe0a526, { roughness: 0.5 })));
  const bezel = new THREE.Mesh(new THREE.BoxGeometry(6.6, 4.9, 0.3), mat(0x2c2c2e));
  bezel.position.set(-0.2, 0, 0.23);
  board.add(bezel);

  // The screen: a canvas drawn like the app on the board.
  const scr = document.createElement("canvas");
  scr.width = 640; scr.height = 480;
  const tex = new THREE.CanvasTexture(scr);
  tex.colorSpace = THREE.SRGBColorSpace;
  tex.anisotropy = 4;
  const screen = new THREE.Mesh(new THREE.PlaneGeometry(5.8, 4.35), new THREE.MeshBasicMaterial({ map: tex }));
  screen.position.set(-0.2, 0, 0.39);
  board.add(screen);

  const acrylic = new THREE.MeshStandardMaterial({ color: 0xdfe8ef, transparent: true, opacity: 0.16, roughness: 0.15, depthWrite: false });
  for (const z of [0.62, -0.5]) {
    const plate = new THREE.Mesh(new RoundedBoxGeometry(9.6, 6.0, 0.3, 3, 0.25), acrylic);
    plate.position.z = z;
    board.add(plate);
  }
  const brass = mat(0xc9a24a, { metalness: 0.8, roughness: 0.35 });
  for (const [x, y] of [[-4.4, 2.6], [4.4, 2.6], [-4.4, -2.6], [4.4, -2.6]]) {
    const post = new THREE.Mesh(new THREE.CylinderGeometry(0.16, 0.16, 1.2, 12), brass);
    post.rotation.x = Math.PI / 2;
    post.position.set(x, y, 0.06);
    board.add(post);
  }
  // CN1 on the underside, where the sensor cable leaves.
  const cn1 = new THREE.Mesh(new THREE.BoxGeometry(0.7, 0.3, 0.35), mat(0xf2efe6));
  cn1.position.set(3.1, -1.6, -0.28);
  board.add(cn1);
  board.position.y = 0.6;

  // ---- floor
  const floor = new THREE.Mesh(new THREE.CircleGeometry(12, 96), new THREE.MeshBasicMaterial({ color: 0x2c2c2e }));
  floor.rotation.x = -Math.PI / 2;
  floor.position.y = -3.6;
  scene.add(floor);

  // ---- two DHT22 modules on the floor, wired back to CN1
  const sensors = [-1, 1].map((side, i) => {
    const g = new THREE.Group();
    g.position.set(side * 3.6 + 1.2, -3.6, 4.2);
    g.rotation.y = -side * 0.35;
    scene.add(g);
    const pcb = new THREE.Mesh(new THREE.BoxGeometry(1.9, 0.12, 1.2), mat(0x1e5fa8));
    pcb.position.y = 0.06;
    g.add(pcb);
    const body = new THREE.Mesh(new RoundedBoxGeometry(1.5, 2.4, 0.8, 3, 0.08), mat(0xf4f4f2, { roughness: 0.8 }));
    body.position.y = 1.32;
    g.add(body);
    const slot = mat(0x8e8e93);
    for (let r = 0; r < 5; r++) for (let c = 0; c < 3; c++) {
      const s = new THREE.Mesh(new THREE.BoxGeometry(0.28, 0.2, 0.02), slot);
      s.position.set(-0.38 + c * 0.38, 2.1 - r * 0.36, 0.41);
      g.add(s);
    }
    const ring = new THREE.Mesh(new THREE.RingGeometry(1.45, 1.6, 64),
      new THREE.MeshBasicMaterial({ color: IDENT[i], transparent: true, opacity: 0.3, side: THREE.DoubleSide }));
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.02;
    g.add(ring);
    // Wires: 3.3 V, data, GND, from the module's back to CN1.
    const from = new THREE.Vector3();
    cn1.getWorldPosition(from);
    const wires = [0xff453a, IDENT[i], 0x3a3a3c];
    wires.forEach((color, k) => {
      const end = new THREE.Vector3((k - 1) * 0.25, 0.3, -0.62).applyMatrix4(g.matrixWorld.clone().compose(g.position, g.quaternion, g.scale));
      const a = from.clone().add(new THREE.Vector3((k - 1) * 0.08 + i * 0.1, 0, 0));
      const curve = new THREE.CatmullRomCurve3([
        a, a.clone().add(new THREE.Vector3(0, -1.2, -0.6)),
        new THREE.Vector3((a.x + end.x) / 2, -3.45, (a.z + end.z) / 2 - 0.5), end,
      ]);
      g.parent.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 48, 0.045, 8), mat(color, { roughness: 0.4 })));
    });
    return { g, ring, body, pulse: 0, last: null };
  });

  // ---- live data onto the model
  function drawScreen() {
    const c = scr.getContext("2d"), W = scr.width, H = scr.height, k = W / 320;
    const font = (w, px) => `${w} ${px * k}px Inter, "Segoe UI", system-ui, sans-serif`;
    c.fillStyle = "#000"; c.fillRect(0, 0, W, H);
    c.fillStyle = "#0A84FF"; c.font = font(500, 15); c.textBaseline = "middle";
    c.fillText("‹ Home", 12 * k, 16 * k);
    c.fillStyle = "#fff"; c.font = font(600, 17); c.textAlign = "center";
    c.fillText("Sensors", W / 2, 16 * k);
    c.textAlign = "left";
    [0, 1].forEach((i) => {
      const x = (12 + i * 153) * k, y = 44 * k, w = 143 * k, h = 176 * k;
      c.fillStyle = "#1C1C1E";
      c.beginPath(); c.roundRect(x, y, w, h, 12 * k); c.fill();
      const r = S.readings[i], ok = r && r.state === "ok";
      c.fillStyle = "#fff"; c.font = font(600, 13);
      c.fillText(clip(c, S.names[i], 72 * k), x + 10 * k, y + 18 * k);
      c.fillStyle = "#98989F"; c.font = font(400, 12);
      c.textAlign = "right"; c.fillText(r && r.read ? r.read : "--:--:--", x + w - 10 * k, y + 18 * k); c.textAlign = "left";
      c.fillText(r ? r.label : "Reading", x + 10 * k, y + 36 * k);
      c.fillStyle = "#fff"; c.font = font(600, 22);
      c.fillText(ok ? `${r.temp.toFixed(1)} °C` : "--", x + 10 * k, y + 64 * k);
      bar(c, x + 10 * k, y + 82 * k, w - 20 * k, ok ? (r.temp + 10) / 60 : null, k);
      c.fillStyle = "#fff"; c.font = font(600, 17);
      c.fillText(ok ? `${r.hum.toFixed(1)} %` : "--", x + 10 * k, y + 112 * k);
      bar(c, x + 10 * k, y + 128 * k, w - 20 * k, ok ? r.hum / 100 : null, k);
    });
    tex.needsUpdate = true;
  }
  const clip = (c, s, w) => { let t = s; while (t.length > 1 && c.measureText(t).width > w) t = t.slice(0, -1); return t === s ? s : t.slice(0, -1) + "…"; };
  function bar(c, x, y, w, f, k) {
    if (f === null) return;
    c.fillStyle = "#3A3A3C"; c.beginPath(); c.roundRect(x, y, w, 6 * k, 3 * k); c.fill();
    c.fillStyle = "#0A84FF"; c.beginPath(); c.roundRect(x, y, Math.max(6 * k, w * Math.max(0, Math.min(1, f))), 6 * k, 3 * k); c.fill();
  }

  function onData() {
    sensors.forEach((s, i) => {
      const r = S.readings[i];
      const key = r && r.state === "ok" ? r.read + r.temp + r.hum : null;
      if (key && key !== s.last) s.pulse = 1;
      s.last = key;
      const ok = !!r && r.state === "ok";
      s.body.material.color.setHex(ok ? 0xf4f4f2 : 0x8e8e93);
      s.ring.material.color.setHex(ok ? IDENT[i] : 0x636366);
      tags[i].innerHTML = "";
      const sw = document.createElement("span");
      sw.className = "sw"; sw.style.background = ok ? `var(--s${i + 1})` : "var(--label-2)";
      tags[i].append(sw, document.createTextNode(ok ? `${r.temp.toFixed(1)} °C · ${Math.round(r.hum)} %` : (r ? r.label : S.names[i])));
      tags[i].classList.toggle("on", S.online);
    });
    drawScreen();
  }
  addEventListener("sensors", onData);
  document.fonts && document.fonts.ready.then(drawScreen);
  onData();

  function resize() {
    const w = stage.clientWidth, h = stage.clientHeight;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.fov = w < 520 ? 30 : 32;
    camera.updateProjectionMatrix();
  }
  new ResizeObserver(resize).observe(stage);
  resize();

  const v = new THREE.Vector3();
  let prev = performance.now();
  renderer.setAnimationLoop((now) => {
    const dt = Math.min((now - prev) / 1000, 0.1);
    prev = now;
    controls.update();
    sensors.forEach((s, i) => {
      s.pulse = Math.max(0, s.pulse - dt * 1.2);
      s.ring.material.opacity = 0.3 + 0.6 * s.pulse;
      s.ring.scale.setScalar(1 + 0.25 * (1 - s.pulse) * (s.pulse > 0 ? 1 : 0));
      v.set(0, 3.1, 0).applyMatrix4(s.g.matrixWorld).project(camera);
      tags[i].style.left = ((v.x + 1) / 2) * stage.clientWidth + "px";
      tags[i].style.top = ((1 - v.y) / 2) * stage.clientHeight + "px";
    });
    renderer.render(scene, camera);
  });
}

try {
  start();
} catch (e) {
  stage.hidden = true; // no WebGL: the cards and charts are the page
}
