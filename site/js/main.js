// Entry point. Everything here works without the animation libraries.
// Motion (GSAP, Lenis) loads after the first paint, and only when the
// visitor hasn't asked for less motion.

const reduce = matchMedia("(prefers-reduced-motion: reduce)");
const saveData = navigator.connection?.saveData === true;

// ---------- clips play only while on screen ----------
const clips = document.querySelectorAll("video[data-autoplay]");
if (clips.length && "IntersectionObserver" in window) {
  const io = new IntersectionObserver(
    (entries) => {
      for (const { target, isIntersecting } of entries) {
        if (isIntersecting && !reduce.matches) target.play().catch(() => {});
        else target.pause();
      }
    },
    { threshold: 0.35 },
  );
  clips.forEach((v) => io.observe(v));
}

// ---------- phone download bar ----------
// Shown after the hero button scrolls away. It steps aside while a download
// button or the plans are on screen, so it never covers a purchase button.
{
  const bar = document.querySelector("[data-sticky-cta]");
  const heroCta = document.querySelector(".hero .btn-large");
  const hide = [document.querySelector(".final .btn"), document.querySelector(".plan-list")];
  if (bar && heroCta && hide.every(Boolean) && "IntersectionObserver" in window) {
    const seen = new Map();
    const show = () => {
      const hero = seen.get(heroCta);
      const pastHero = Boolean(hero && !hero.visible && hero.above);
      const covered = hide.some((el) => seen.get(el)?.visible);
      const on = pastHero && !covered;
      bar.classList.toggle("on", on);
      bar.inert = !on;
    };
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          seen.set(e.target, { visible: e.isIntersecting, above: e.boundingClientRect.top < 0 });
        }
        show();
      },
      { threshold: 0 },
    );
    [heroCta, ...hide].forEach((el) => io.observe(el));
  }
}

// ---------- theme switch with a circular wipe ----------
{
  const btn = document.querySelector("[data-theme-toggle]");
  const root = document.documentElement;
  const systemDark = matchMedia("(prefers-color-scheme: dark)");
  const effective = () => root.dataset.theme || (systemDark.matches ? "dark" : "light");
  const label = () => {
    const next = effective() === "dark" ? "light" : "dark";
    btn.textContent = `Switch to ${next}`;
  };
  label();

  // Switches the theme with a circular wipe that grows from (x, y). Calls that
  // arrive while a wipe is running are folded into one, applied afterwards.
  let busy = false;
  let queued = null;
  const setTheme = (next, x, y) => {
    if (next === effective()) return;
    if (busy) {
      queued = [next, x, y];
      return;
    }
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    const apply = () => {
      root.dataset.theme = next;
      label();
      btn?.setAttribute("aria-pressed", String(next === "dark"));
    };
    if (!document.startViewTransition || reduce.matches) {
      apply();
      return;
    }
    busy = true;
    const transition = document.startViewTransition(apply);
    transition.ready.then(() => {
      root.animate(
        { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
        {
          duration: 850,
          easing: "cubic-bezier(0.2, 0.7, 0.1, 1)",
          pseudoElement: "::view-transition-new(root)",
        },
      );
    });
    transition.finished.finally(() => {
      busy = false;
      if (queued) {
        const q = queued;
        queued = null;
        setTheme(...q);
      }
    });
  };

  let chosen = false; // the visitor used the switch themselves
  btn?.addEventListener("click", (e) => {
    chosen = true;
    const rect = btn.getBoundingClientRect();
    // Keyboard clicks report 0,0, so use the button's centre instead
    const x = e.clientX || rect.left + rect.width / 2;
    const y = e.clientY || rect.top + rect.height / 2;
    setTheme(effective() === "dark" ? "light" : "dark", x, y);
  });

  // "Your look" shows the other theme while it is on screen, then puts the
  // page back. It stops for good once the visitor picks a theme.
  let original = null;
  window.JoyTheme = {
    preview(on) {
      if (chosen) return;
      original ??= effective();
      const x = innerWidth / 2;
      const y = innerHeight * 0.6;
      setTheme(on ? (original === "dark" ? "light" : "dark") : original, x, y);
    },
  };
}

// ---------- greeting in 13 languages ----------
{
  const el = document.querySelector("[data-welcome]");
  const WORDS = [
    ["Welcome", "en"],
    ["Bienvenido", "es"],
    ["Bienvenue", "fr"],
    ["Willkommen", "de"],
    ["Benvenuto", "it"],
    ["Bem-vindo", "pt-BR"],
    ["Добро пожаловать", "ru"],
    ["Hoş geldiniz", "tr"],
    ["مرحبا", "ar"],
    ["स्वागत है", "hi"],
    ["ようこそ", "ja"],
    ["환영합니다", "ko"],
    ["欢迎", "zh-Hans"],
  ];
  const show = (i) => {
    const [text, lang] = WORDS[i];
    el.textContent = text;
    el.lang = lang;
    el.dir = lang === "ar" ? "rtl" : "ltr";
  };
  if (el) {
    show(0);
    if (!reduce.matches && "IntersectionObserver" in window) {
      let i = 0;
      let timer = 0;
      const tick = () => {
        el.classList.add("is-out");
        setTimeout(() => {
          i = (i + 1) % WORDS.length;
          show(i);
          el.classList.remove("is-out");
        }, 500);
      };
      new IntersectionObserver(([entry]) => {
        clearInterval(timer);
        if (entry.isIntersecting) timer = setInterval(tick, 2600);
      }).observe(el);
    }
  }
}

// ---------- the animation layer ----------
// Scripts are loaded in order, one after another, once the page has loaded.
const SCRIPTS = [
  "vendor/gsap.min.js",
  "vendor/ScrollTrigger.min.js",
  "vendor/SplitText.min.js",
  "vendor/lenis.min.js",
];

const loadScript = (src) =>
  new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = src;
    s.onload = resolve;
    s.onerror = reject;
    document.head.append(s);
  });

async function boot() {
  // The light field draws whenever WebGL is available and the visitor
  // hasn't asked for less motion or saved data.
  const bg = document.querySelector(".bg-light");
  let shader = null;
  if (bg && !reduce.matches && !saveData) shader = JoyShader.initLight(bg);

  if (reduce.matches) return;
  try {
    for (const src of SCRIPTS) await loadScript(src);
    await loadScript("js/scenes.js");
    JoyScenes.init({ shader });
    document.documentElement.classList.add("cine");
  } catch (err) {
    // The page is already complete without motion, so just stay static.
    console.warn("JoyOS page: motion layer not loaded", err);
  }
}

JoyDemo.initDemo(document.querySelector("[data-demo]"));

if (document.readyState === "complete") boot();
else addEventListener("load", boot, { once: true });
