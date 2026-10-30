// JoyOS focus engine. Rust sends controller moves here; this file decides which
// element is "next" from where things actually are on screen, so layouts never
// need hand-written focus maps. Anything with [data-nav] can take focus.
//
// Rust → JS: {t:"move",d}, {t:"activate"}, {t:"page",d}, {t:"tilt",x,y}, {t:"prefs",...}, {t:"sound",s}
// JS → Rust: {t:"back"}, {t:"tab",d}, {t:"quick"}, {t:"more"}, {t:"search"}, {t:"slide",id,d}, {t:"type",k}, {t:"keyboard"},
//            {t:"recovery"}, {t:"os-motion",reduced}

// Script errors show up in JoyOS's own log instead of disappearing.
window.addEventListener("error", (e) => dioxus.send({ t: "log", m: `${e.message} @ ${e.filename}:${e.lineno}` }));
window.addEventListener("unhandledrejection", (e) => dioxus.send({ t: "log", m: String(e.reason) }));

// For tools/cdp.mjs, which can't reach the eval's own `dioxus` handle.
window.joySend = (msg) => dioxus.send(msg);

const prefs = { sounds: true, reduced: false, lang: "en", rtl: false };
// Screens are rebuilt on every tab change, so focus is remembered by what an
// element is (its id or label), not by the element itself.
const memory = new Map();
const rowMemory = new Map();
const navKey = (el) => el.dataset.id || el.getAttribute("aria-label") || el.textContent.trim().slice(0, 60);
const findByKey = (root, key) => key && candidates(root).find((el) => navKey(el) === key);

// Tell Rust what the system asks for, so first-run setup can start with less motion.
dioxus.send({ t: "os-motion", reduced: matchMedia("(prefers-reduced-motion: reduce)").matches });

// The lens filter used by `.glass.lens`: low-frequency noise bends what's behind
// the glass, like looking through a thick pane.
document.body.insertAdjacentHTML("beforeend", `
  <svg width="0" height="0" style="position:absolute" aria-hidden="true">
    <filter id="joy-lens" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.004 0.009" numOctaves="2" seed="11" result="noise"/>
      <feGaussianBlur in="noise" stdDeviation="2" result="soft"/>
      <feDisplacementMap in="SourceGraphic" in2="soft" scale="38" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
  </svg>`);

// ---------- sound ----------
let audio;
function sound(kind) {
  if (!prefs.sounds) return;
  try {
    audio ??= new AudioContext();
    if (audio.state === "suspended") audio.resume();
    const now = audio.currentTime;
    const tone = (freq, start, length, gain, type = "sine") => {
      const osc = audio.createOscillator();
      const amp = audio.createGain();
      osc.type = type;
      osc.frequency.setValueAtTime(freq, now + start);
      amp.gain.setValueAtTime(0, now + start);
      amp.gain.linearRampToValueAtTime(gain, now + start + 0.004);
      amp.gain.exponentialRampToValueAtTime(0.0001, now + start + length);
      osc.connect(amp).connect(audio.destination);
      osc.start(now + start);
      osc.stop(now + start + length + 0.02);
    };
    if (kind === "tick") tone(2400, 0, 0.035, 0.018);
    else if (kind === "select") { tone(880, 0, 0.09, 0.05); tone(1320, 0.045, 0.12, 0.04); }
    else if (kind === "back") { tone(990, 0, 0.07, 0.04); tone(660, 0.04, 0.1, 0.035); }
    else if (kind === "bump") tone(140, 0, 0.09, 0.07, "triangle");
    else if (kind === "open") { tone(520, 0, 0.16, 0.035); tone(780, 0.05, 0.2, 0.03); tone(1170, 0.1, 0.26, 0.022); }
  } catch (_) {}
}

// ---------- scopes ----------
// The top-most [data-layer] owns focus; otherwise the main view does.
function scope() {
  const layers = document.querySelectorAll("[data-layer]");
  return layers.length ? layers[layers.length - 1] : document.querySelector("[data-main]") ?? document.body;
}
const scopeKey = (root) => root.dataset.layer ?? root.dataset.main ?? "root";

function visible(el) {
  if (el.disabled || el.closest("[inert]")) return false;
  const r = el.getBoundingClientRect();
  return r.width > 0 && r.height > 0;
}
const candidates = (root) => [...root.querySelectorAll("[data-nav]")].filter(visible);

function current(root = scope()) {
  const a = document.activeElement;
  return a && a !== document.body && root.contains(a) && a.matches("[data-nav]") ? a : null;
}

let settleQueued = false;
function settle() {
  if (settleQueued) return;
  settleQueued = true;
  requestAnimationFrame(() => {
    settleQueued = false;
    const root = scope();
    if (current(root)) return;
    const target =
      findByKey(root, memory.get(scopeKey(root))) ||
      root.querySelector("[data-autofocus]") ||
      candidates(root)[0];
    if (target) place(target, false);
  });
}

// ---------- moving ----------
// The floating keyboard's window never takes focus, so :focus never matches
// there. Mirror it as a class that styles can use.
let marked = null;
function mark(el) {
  if (marked === el) return;
  marked?.classList.remove("is-focus");
  marked = el;
  el?.classList.add("is-focus");
}

function place(el, withSound = true) {
  el.focus({ preventScroll: true });
  mark(el);
  reveal(el);
  if (withSound) sound("tick");
}

// Only real scroll containers (overflow: auto/scroll) move. scrollIntoView would
// also shift overflow:hidden layers, which slides whole screens out of place.
function reveal(el) {
  const behavior = prefs.reduced ? "auto" : "smooth";
  for (let box = el.parentElement; box && box !== document.body; box = box.parentElement) {
    const style = getComputedStyle(box);
    const sx = /auto|scroll/.test(style.overflowX) && box.scrollWidth > box.clientWidth;
    const sy = /auto|scroll/.test(style.overflowY) && box.scrollHeight > box.clientHeight;
    if (!sx && !sy) continue;
    const b = box.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    const to = {};
    // Rows keep the focused tile near the middle, like a TV.
    if (sx) to.left = box.scrollLeft + (r.left + r.width / 2) - (b.left + b.width / 2);
    // Lists only move when the item gets close to an edge.
    if (sy) {
      const margin = Math.min(16, b.height / 8);
      if (r.top < b.top + margin) to.top = box.scrollTop - (b.top + margin - r.top);
      else if (r.bottom > b.bottom - margin) to.top = box.scrollTop + (r.bottom - (b.bottom - margin));
    }
    box.scrollTo({ ...to, behavior });
  }
}

function rect(el) {
  const r = el.getBoundingClientRect();
  return { l: r.left, r: r.right, t: r.top, b: r.bottom, cx: (r.left + r.right) / 2, cy: (r.top + r.bottom) / 2 };
}

function best(from, dir, list) {
  const a = rect(from);
  let winner = null;
  let winnerScore = Infinity;
  for (const el of list) {
    if (el === from) continue;
    const b = rect(el);
    let primary, overlap, offset;
    if (dir === "right") { primary = b.l - a.r; overlap = Math.min(a.b, b.b) - Math.max(a.t, b.t); offset = Math.abs(b.cy - a.cy); if (b.cx <= a.cx + 1) continue; }
    if (dir === "left") { primary = a.l - b.r; overlap = Math.min(a.b, b.b) - Math.max(a.t, b.t); offset = Math.abs(b.cy - a.cy); if (b.cx >= a.cx - 1) continue; }
    if (dir === "down") { primary = b.t - a.b; overlap = Math.min(a.r, b.r) - Math.max(a.l, b.l); offset = Math.abs(b.cx - a.cx); if (b.cy <= a.cy + 1) continue; }
    if (dir === "up") { primary = a.t - b.b; overlap = Math.min(a.r, b.r) - Math.max(a.l, b.l); offset = Math.abs(b.cx - a.cx); if (b.cy >= a.cy - 1) continue; }
    primary = Math.max(primary, 0);
    // Things lined up with us win over things that are merely close.
    const score = primary + (overlap > 0 ? offset * 0.15 : offset * 2.2 + 400);
    if (score < winnerScore) { winnerScore = score; winner = el; }
  }
  return winner;
}

function bump(el, dir) {
  if (!el) return;
  sound("bump");
  if (prefs.reduced) return;
  el.classList.remove("bump-up", "bump-down", "bump-left", "bump-right");
  void el.offsetWidth;
  el.classList.add(`bump-${dir}`);
  setTimeout(() => el.classList.remove(`bump-${dir}`), 320);
}

function move(dir, quiet = false) {
  const root = scope();
  const from = current(root);
  if (!from) return settle();

  // Sliders use left/right for their value, like a real control.
  if (from.dataset.slider && (dir === "left" || dir === "right")) {
    const sign = (dir === "right") !== prefs.rtl ? 1 : -1;
    dioxus.send({ t: "slide", id: from.dataset.slider, d: sign });
    sound("tick");
    return;
  }

  let target = best(from, dir, candidates(root));
  // Entering a different row goes back to where you were in it, like a TV.
  if (target && (dir === "up" || dir === "down")) {
    const row = target.closest("[data-row]");
    if (row && row !== from.closest("[data-row]")) {
      const last = findByKey(row, rowMemory.get(`${scopeKey(root)}/${row.dataset.row}`));
      if (last) target = last;
    }
  }
  if (target) place(target, !quiet);
  else if (!quiet) bump(from, dir);
  return !!target;
}

function activate() {
  const el = current();
  if (!el) return settle();
  el.classList.remove("pressed");
  void el.offsetWidth;
  el.classList.add("pressed");
  setTimeout(() => el.classList.remove("pressed"), 260);
  sound(el.dataset.sound ?? "select");
  el.click();
}

// ---------- tilt ----------
let tilted = null;
function tilt(x, y) {
  const el = current();
  if (tilted && tilted !== el) { tilted.style.removeProperty("--tx"); tilted.style.removeProperty("--ty"); }
  tilted = el;
  if (!el || prefs.reduced) return;
  el.style.setProperty("--tx", x.toFixed(3));
  el.style.setProperty("--ty", (-y).toFixed(3));
}

// ---------- clock ----------
// Writes only on change: the MutationObserver below calls this, and an
// unconditional write would trigger it again forever.
function tickClock() {
  const now = new Date();
  const set = (sel, text) => {
    for (const el of document.querySelectorAll(sel)) if (el.textContent !== text) el.textContent = text;
  };
  set("[data-clock]", now.toLocaleTimeString(prefs.lang, { hour: "numeric", minute: "2-digit" }));
  set("[data-date]", now.toLocaleDateString(prefs.lang, { weekday: "long", day: "numeric", month: "long" }));
}
setInterval(tickClock, 5000);

// ---------- wiring ----------
// A new clock only needs filling in once; the timer keeps it current after that.
new MutationObserver(() => {
  settle();
  if (document.querySelector("[data-clock]:empty, [data-date]:empty")) tickClock();
}).observe(document.body, { childList: true, subtree: true });

document.addEventListener("focusin", (e) => {
  const el = e.target;
  if (!(el instanceof HTMLElement) || !el.matches("[data-nav]")) return;
  mark(el);
  const root = scope();
  if (!root.contains(el)) return;
  memory.set(scopeKey(root), navKey(el));
  const row = el.closest("[data-row]");
  if (row) rowMemory.set(`${scopeKey(root)}/${row.dataset.row}`, navKey(el));
});

// The mouse works too: hovering focuses, like moving with the stick.
document.addEventListener("pointermove", (e) => {
  const el = e.target instanceof Element ? e.target.closest("[data-nav]") : null;
  if (el && el !== document.activeElement && scope().contains(el)) { el.focus({ preventScroll: true }); mark(el); }
});

const KEY_DIRS = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
document.addEventListener("keydown", (e) => {
  const typing = !!document.querySelector("[data-layer=keyboard]");
  if (KEY_DIRS[e.key]) { e.preventDefault(); return move(KEY_DIRS[e.key]); }
  if (e.key === "Enter" || (e.key === " " && !typing)) { e.preventDefault(); return activate(); }
  if (typing && !e.ctrlKey && !e.altKey && (e.key.length === 1 || e.key === "Backspace")) {
    e.preventDefault();
    return dioxus.send({ t: "type", k: e.key === "Backspace" ? "\b" : e.key });
  }
  const send = (msg) => { e.preventDefault(); dioxus.send(msg); };
  // The keyboard's way back to Windows, matching LB + RB + View + Start on a controller.
  if (e.ctrlKey && e.altKey && e.shiftKey && e.code === "KeyD") return send({ t: "recovery" });
  if (e.key === "Escape" || e.key === "Backspace" || e.key === "BrowserBack") return send({ t: "back" });
  if (e.key === "PageUp" || (e.key === "Tab" && e.shiftKey)) return send({ t: "tab", d: -1 });
  if (e.key === "PageDown" || e.key === "Tab") return send({ t: "tab", d: 1 });
  if (e.key === "F1" || e.key === "ContextMenu") return send({ t: "quick" });
  // Letter shortcuts are plain presses only, so Ctrl+E or a held key doesn't fire them.
  if (e.ctrlKey || e.altKey || e.metaKey || e.repeat) return;
  if (e.key === "q") return send({ t: "tab", d: -1 });
  if (e.key === "e") return send({ t: "tab", d: 1 });
  if (e.key === "m") return send({ t: "quick" });
  if (e.key === "/" || e.key === "f") return send({ t: "search" });
  if (e.key === "x") return send({ t: "more" });
}, true);

document.addEventListener("contextmenu", (e) => e.preventDefault());

settle();
tickClock();

while (true) {
  const msg = await dioxus.recv();
  switch (msg.t) {
    case "move": move(msg.d); break;
    case "activate": activate(); break;
    case "page": {
      // One tick for the whole jump, not four.
      let moved = false;
      for (let i = 0; i < 4; i++) moved = move(msg.d > 0 ? "down" : "up", true) || moved;
      if (moved) sound("tick");
      break;
    }
    case "tilt": tilt(msg.x, msg.y); break;
    case "sound": sound(msg.s); break;
    case "prefs":
      Object.assign(prefs, msg);
      document.documentElement.lang = prefs.lang;
      document.documentElement.dir = prefs.rtl ? "rtl" : "ltr";
      tickClock();
      break;
  }
}
