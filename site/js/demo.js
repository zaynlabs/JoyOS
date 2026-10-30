(() => {
// A small working copy of JoyOS for the page. Keyboard, touch, and the
// Gamepad API all drive it. Nothing is launched: picks show a note instead.

const TABS = [
  { name: "Home", tiles: ["Harbor Run", "Sky Garden", "Velvet Drift", "Ember Tactics"] },
  { name: "Games", tiles: ["Quiet Moor", "Lantern Road", "Glass Orchard", "Tidewater", "Ridge Line", "Night Market"] },
  { name: "Watch", tiles: ["Netflix", "YouTube", "Twitch", "Spotify"] },
  { name: "Apps", tiles: ["Notes", "Photos", "Mail", "Calendar"] },
  { name: "Settings", tiles: ["Look", "Language", "Controller", "Family"] },
];

// Warm and teal hues only, so the tiles stay on-brand
const HUES = [200, 185, 30, 18, 170, 45, 210, 25];

function initDemo(root) {
  if (!root) return;
  const screen = root.querySelector("[data-screen]");
  const tabsEl = root.querySelector("[data-tabs]");
  const gridEl = root.querySelector("[data-grid]");
  const titleEl = root.querySelector("[data-tab-title]");
  const menu = root.querySelector("[data-menu]");
  const toastEl = root.querySelector("[data-toast]");
  const chip = root.querySelector("[data-chip]");
  const volume = root.querySelector("[data-volume]");
  const volumeOut = root.querySelector("[data-vol-out]");
  const mouseBtn = root.querySelector("[data-mouse]");
  const mouseState = root.querySelector("[data-mouse-state]");

  let tab = 0;
  let focus = 0;
  let menuOpen = false;
  let toastTimer = 0;

  // ---------- rendering ----------
  TABS.forEach((t, i) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = t.name;
    b.setAttribute("aria-pressed", "false");
    b.addEventListener("click", () => setTab(i));
    tabsEl.append(b);
  });

  function renderTiles() {
    const set = TABS[tab].tiles;
    titleEl.textContent = TABS[tab].name;
    gridEl.replaceChildren(
      ...set.map((name, i) => {
        const tile = document.createElement("button");
        tile.type = "button";
        tile.className = "tile";
        tile.style.setProperty("--h", HUES[(i + tab * 3) % HUES.length]);
        const title = document.createElement("span");
        title.className = "tile-title";
        title.textContent = name;
        const tag = document.createElement("span");
        tag.className = "tile-tag";
        tag.textContent = "Example";
        tile.append(title, tag);
        tile.addEventListener("click", () => {
          focus = i;
          mark();
          toast(`Example only. The real app would open ${name}.`);
        });
        tile.addEventListener("focus", () => {
          focus = i;
          mark();
        });
        return tile;
      }),
    );
    focus = Math.min(focus, set.length - 1);
    mark();
  }

  function renderTabs() {
    [...tabsEl.children].forEach((b, i) => b.setAttribute("aria-pressed", String(i === tab)));
  }

  function mark() {
    const tiles = gridEl.children;
    [...tiles].forEach((t, i) => t.classList.toggle("is-focus", i === focus));
  }

  function setTab(next) {
    // Rebuilding the tiles drops focus, so keep it inside the demo
    const hadFocus = screen.contains(document.activeElement);
    tab = (next + TABS.length) % TABS.length;
    focus = 0;
    renderTabs();
    renderTiles();
    if (hadFocus) gridEl.children[focus]?.focus({ preventScroll: true });
  }

  // ---------- movement ----------
  const columns = () => {
    const cols = getComputedStyle(gridEl).gridTemplateColumns.split(" ").length;
    return Math.max(1, cols);
  };

  function moveFocus(dx, dy) {
    const count = gridEl.children.length;
    if (!count) return;
    const cols = columns();
    let next = focus + dx + dy * cols;
    next = Math.max(0, Math.min(count - 1, next));
    focus = next;
    mark();
    gridEl.children[focus].focus({ preventScroll: true });
  }

  function menuItems() {
    return [...menu.querySelectorAll("[data-m]")];
  }
  let menuFocus = 0;

  function menuMove(dy) {
    const items = menuItems();
    menuFocus = (menuFocus + dy + items.length) % items.length;
    items.forEach((el, i) => el.classList.toggle("is-focus", i === menuFocus));
    items[menuFocus].focus({ preventScroll: true });
  }

  function menuAdjust(dx) {
    const items = menuItems();
    const el = items[menuFocus];
    if (el !== volume) return;
    volume.value = Math.max(0, Math.min(100, Number(volume.value) + dx * 5));
    volumeOut.textContent = volume.value;
  }

  function openMenu() {
    menuOpen = true;
    menu.hidden = false;
    menuFocus = 0;
    menuMove(0);
  }

  function closeMenu() {
    menuOpen = false;
    menu.hidden = true;
    menuItems().forEach((el) => el.classList.remove("is-focus"));
    gridEl.children[focus]?.focus({ preventScroll: true });
  }

  function toggleMenu() {
    if (menuOpen) closeMenu();
    else openMenu();
  }

  menu.addEventListener("click", (e) => {
    const act = e.target.closest("[data-act]")?.dataset.act;
    if (e.target.closest("[data-mouse]")) {
      const on = mouseBtn.getAttribute("aria-pressed") !== "true";
      mouseBtn.setAttribute("aria-pressed", String(on));
      mouseState.textContent = on ? "On" : "Off";
      toast(on ? "Mouse mode on. Example only." : "Mouse mode off.");
    } else if (act === "shot") toast("Screenshot taken. Example only.");
    else if (act === "clip") toast("Last 30 seconds saved. Example only.");
    else if (act === "close") closeMenu();
  });
  volume.addEventListener("input", () => {
    volumeOut.textContent = volume.value;
  });

  root.querySelector("[data-open-menu]").addEventListener("click", toggleMenu);

  function toast(text) {
    toastEl.textContent = text;
    toastEl.classList.add("on");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toastEl.classList.remove("on"), 2400);
  }

  // ---------- keyboard ----------
  screen.addEventListener("keydown", (e) => {
    const k = e.key;
    if (menuOpen) {
      if (k === "ArrowDown") (e.preventDefault(), menuMove(1));
      else if (k === "ArrowUp") (e.preventDefault(), menuMove(-1));
      else if (k === "ArrowRight") (e.preventDefault(), menuAdjust(1));
      else if (k === "ArrowLeft") (e.preventDefault(), menuAdjust(-1));
      else if (k === "Escape" || k === "m" || k === "M" || k === "Backspace") (e.preventDefault(), closeMenu());
      return;
    }
    if (k === "ArrowRight") (e.preventDefault(), moveFocus(1, 0));
    else if (k === "ArrowLeft") (e.preventDefault(), moveFocus(-1, 0));
    else if (k === "ArrowDown") (e.preventDefault(), moveFocus(0, 1));
    else if (k === "ArrowUp") (e.preventDefault(), moveFocus(0, -1));
    else if (k === "e" || k === "E") setTab(tab + 1);
    else if (k === "q" || k === "Q") setTab(tab - 1);
    else if (k === "m" || k === "M") openMenu();
  });

  // ---------- controller (Gamepad API, only while the demo is on screen) ----------
  const prev = {};
  let repeatAt = 0;
  let loopId = 0;
  let visible = false;

  const edge = (pad, i) => {
    const now = Boolean(pad.buttons[i]?.pressed);
    const was = prev[i] ?? false;
    prev[i] = now;
    return now && !was;
  };

  // Gamepad button indices: 0 A, 1 B, 4 LB, 5 RB, 9 Start, 12-15 D-pad
  function step(dx, dy) {
    if (menuOpen) {
      if (dy) menuMove(dy);
      if (dx) menuAdjust(dx);
    } else {
      moveFocus(dx, dy);
    }
  }

  function poll() {
    if (!visible) {
      loopId = 0;
      return;
    }
    loopId = requestAnimationFrame(poll);
    const pad = [...navigator.getGamepads()].find(Boolean);
    if (!pad) return;

    const dpad = [
      [12, 0, -1],
      [13, 0, 1],
      [14, -1, 0],
      [15, 1, 0],
    ];
    for (const [b, dx, dy] of dpad) if (edge(pad, b)) step(dx, dy);

    // Left stick repeats while held
    const now = performance.now();
    if (now > repeatAt) {
      const ax = pad.axes[0] ?? 0;
      const ay = pad.axes[1] ?? 0;
      const sx = Math.abs(ax) > 0.5 ? Math.sign(ax) : 0;
      const sy = Math.abs(ay) > 0.5 ? Math.sign(ay) : 0;
      if (sx || sy) {
        step(sx, sy);
        repeatAt = now + 260;
      }
    }

    if (edge(pad, 0)) {
      if (menuOpen) menuItems()[menuFocus]?.click();
      else gridEl.children[focus]?.click();
      haptic(pad);
    }
    if (edge(pad, 1) && menuOpen) closeMenu();
    if (edge(pad, 4)) setTab(tab - 1);
    if (edge(pad, 5)) setTab(tab + 1);
    if (edge(pad, 9)) toggleMenu();
  }

  function haptic(pad) {
    try {
      pad.vibrationActuator?.playEffect("dual-rumble", {
        duration: 40,
        strongMagnitude: 0.25,
        weakMagnitude: 0.15,
      })?.catch?.(() => {});
    } catch (_) {}
  }

  function updateChip() {
    const found = [...navigator.getGamepads()].some(Boolean);
    chip.textContent = found ? "Controller found" : "Using the keyboard";
  }
  addEventListener("gamepadconnected", () => {
    updateChip();
    if (visible && !loopId) loopId = requestAnimationFrame(poll);
  });
  addEventListener("gamepaddisconnected", updateChip);

  // Poll only while the demo is on screen
  new IntersectionObserver(
    ([entry]) => {
      visible = entry.isIntersecting;
      if (visible && !loopId) loopId = requestAnimationFrame(poll);
    },
    { threshold: 0.2 },
  ).observe(root);

  // Start on the first section, with the right chip text
  setTab(0);
  updateChip();
}

window.JoyDemo = { initDemo };
})();
