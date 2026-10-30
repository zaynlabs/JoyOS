(() => {
// The motion layer: GSAP, ScrollTrigger, SplitText and Lenis. Loaded only
// after the page has rendered, and only when the visitor allows motion.
// Every scene is written so the page still reads if this file never runs.

function init({ shader }) {
  const gsap = window.gsap;
  const ST = window.ScrollTrigger;
  const SplitText = window.SplitText;
  gsap.registerPlugin(ST, SplitText);

  const fine = matchMedia("(pointer: fine)").matches;
  const $ = (sel) => document.querySelector(sel);
  const accent = "rgb(226 150 92)";

  // ---------- smooth scroll (mouse and trackpad only; touch stays native) ----------
  if (fine && window.Lenis) {
    const lenis = new window.Lenis({ lerp: 0.085, smoothWheel: true });
    lenis.on("scroll", ST.update);
    gsap.ticker.add((time) => lenis.raf(time * 1000));
    gsap.ticker.lagSmoothing(0);
  }

  // Maps a section's scroll progress (0 to 1) onto a CSS variable. The
  // stylesheet turns that variable into the scene's state.
  const scrubVar = (el, name) =>
    ST.create({
      trigger: el,
      start: "top top",
      end: "bottom bottom",
      onUpdate: (s) => el.style.setProperty(name, s.progress.toFixed(4)),
    });

  // SplitText's line masks clip descenders on big serif type, so give each
  // mask a little room below and pull it back with a negative margin.
  const roomBelow = (split) =>
    split.masks?.forEach((m) => {
      m.style.paddingBottom = "0.16em";
      m.style.marginBottom = "-0.16em";
    });

  // The title card plays for about 1.6s, so the hero starts as it fades
  const lead = Math.max(0, 1.55 - performance.now() / 1000);

  // ---------- 1. Opening: headline, console and controller ----------
  const heroTitle = SplitText.create(".hero-title", { type: "lines", mask: "lines" });
  roomBelow(heroTitle);
  gsap.from(heroTitle.lines, {
    yPercent: 115,
    duration: 1.5,
    ease: "expo.out",
    stagger: 0.14,
    delay: lead,
  });
  gsap.from(".hero .eyebrow, .hero-sub, .hero-actions, .hero .fine, .scroll-cue", {
    opacity: 0,
    y: 22,
    duration: 1.1,
    ease: "expo.out",
    stagger: 0.08,
    delay: lead + 0.55,
  });
  gsap.from(".hero-chip", { opacity: 0, y: 90, duration: 1.8, ease: "expo.out", delay: lead + 0.55 });
  gsap.from(".hero-layer", { opacity: 0, y: 70, duration: 1.8, ease: "expo.out", delay: lead + 0.25 });
  // Console and controller drift apart as the page scrolls away
  gsap.to(".hero-stage", {
    yPercent: 14,
    ease: "none",
    scrollTrigger: { trigger: ".hero", start: "top top", end: "bottom top", scrub: true },
  });
  if (fine) {
    const layer = gsap.quickTo(".hero-layer", "x", { duration: 1.2, ease: "power3" });
    const layerY = gsap.quickTo(".hero-layer", "y", { duration: 1.2, ease: "power3" });
    const chip = gsap.quickTo(".hero-chip", "x", { duration: 1.6, ease: "power3" });
    const chipY = gsap.quickTo(".hero-chip", "y", { duration: 1.6, ease: "power3" });
    $(".hero").addEventListener(
      "pointermove",
      (e) => {
        const nx = e.clientX / innerWidth - 0.5;
        const ny = e.clientY / innerHeight - 0.5;
        layer(nx * -34);
        layerY(ny * -22);
        chip(nx * 60);
        chipY(ny * 40);
      },
      { passive: true },
    );
  }

  // ---------- 2. The push-in: the frame grows until it fills the screen ----------
  const dolly = $(".dolly");
  const frame = dolly.querySelector(".dolly-frame");
  scrubVar(dolly, "--d");
  gsap.fromTo(
    frame,
    { scale: 0.82, borderRadius: 36 },
    {
      // Scale until the frame covers the whole screen, on either axis
      scale: () => Math.max(innerWidth / frame.offsetWidth, innerHeight / frame.offsetHeight),
      borderRadius: 0,
      ease: "none",
      scrollTrigger: {
        trigger: dolly,
        start: "top top",
        end: "bottom bottom",
        scrub: true,
        invalidateOnRefresh: true,
      },
    },
  );
  if (fine) {
    const rx = gsap.quickTo(frame, "rotationX", { duration: 1, ease: "power3" });
    const ry = gsap.quickTo(frame, "rotationY", { duration: 1, ease: "power3" });
    $(".dolly-stage").addEventListener(
      "pointermove",
      (e) => {
        const r = frame.getBoundingClientRect();
        rx(-((e.clientY - r.top) / r.height - 0.5) * 6);
        ry(((e.clientX - r.left) / r.width - 0.5) * 8);
      },
      { passive: true },
    );
  }

  // ---------- 3. Put the mouse down: words light up, the controller draws and glows ----------
  const mouse = $(".mouse");
  const mouseWords = SplitText.create(".mouse-title", { type: "words" });
  gsap
    .timeline({ scrollTrigger: { trigger: mouse, start: "top top", end: "bottom bottom", scrub: true } })
    .fromTo(mouseWords.words, { opacity: 0.12 }, { opacity: 1, ease: "none", stagger: { amount: 0.6 } }, 0)
    .fromTo(".pad-draw", { strokeDashoffset: 1 }, { strokeDashoffset: 0, ease: "none", stagger: { amount: 0.3 } }, 0.1)
    .fromTo(".pad-lit > *", { opacity: 0 }, { opacity: 1, ease: "none", stagger: 0.07, duration: 0.2 }, 0.7)
    .to(".mouse-svg", { x: () => innerWidth * 0.45, y: 160, rotate: 28, opacity: 0, ease: "none" }, 0.25);

  // ---------- 4. Press Start: the quick menu clip, driven by --p ----------
  scrubVar($(".pin"), "--p");

  // ---------- 5. Try it: the demo's chapter reveals with the rest ----------
  // (its behaviour lives in demo.js; only the reveal is handled here)

  // ---------- 6. One place: giant store names sweep sideways, filling in as they pass ----------
  ST.matchMedia({
    "(min-width: 761px)": () => {
      const run = $(".run");
      const track = run.querySelector(".track");
      const dist = () => Math.max(0, track.scrollWidth - innerWidth);
      const sweep = gsap.to(track, {
        x: () => -dist(),
        ease: "none",
        scrollTrigger: {
          trigger: run,
          start: "top top",
          end: () => `+=${dist()}`,
          pin: true,
          scrub: 0.8,
          anticipatePin: 1,
          invalidateOnRefresh: true,
        },
      });
      run.querySelectorAll(".run-item:not(.run-end)").forEach((item) => {
        item.style.setProperty("--fill", "0");
        ST.create({
          trigger: item,
          containerAnimation: sweep,
          start: "left 82%",
          end: "left 46%",
          onUpdate: (s) => item.style.setProperty("--fill", s.progress.toFixed(3)),
        });
      });
    },
  });

  // ---------- 7. Any app: the pointer glides, the keys light, the word types ----------
  const apps = $("#apps");
  const mock = apps.querySelector(".mock");
  const typed = mock.querySelector("[data-typed]");
  const keyFor = (letter) => mock.querySelector(`[data-k="${letter}"]`);
  const word = "couch";
  const apt = gsap.timeline({
    scrollTrigger: { trigger: apps, start: "top 65%", end: "bottom 45%", scrub: 0.6 },
  });
  apt.fromTo(".mock-ptr", { left: "8%", top: "74%" }, { left: "46%", top: "34%", ease: "power1.inOut", duration: 2 }, 0);
  [...word].forEach((letter, i) => {
    const t = 2 + i * 0.6;
    apt.set(typed, { textContent: word.slice(0, i + 1) }, t);
    // Colour tweens scrub both ways; class toggling doesn't
    apt.to(keyFor(letter), { backgroundColor: accent, color: "#0b0b0d", duration: 0.2, ease: "none" }, t);
  });
  apt.to(".mock-ptr", { left: "86%", top: "78%", ease: "power1.inOut", duration: 1.6 }, 5.4);

  // ---------- 8. Watch: three screens fan apart, the middle one drifts ----------
  const fan = $(".fan");
  fan.style.setProperty("--f", "0");
  ST.create({
    trigger: fan,
    start: "top 88%",
    end: "top 30%",
    onUpdate: (s) => fan.style.setProperty("--f", s.progress.toFixed(3)),
  });

  // ---------- 9. Family: the ring counts down from an hour to five minutes, then buzzes ----------
  const arc = $(".ring-arc");
  const readout = $("[data-ring]");
  const dots = gsap.utils.toArray(".pin-dots i");
  const clock = { v: 60 };
  gsap
    .timeline({ scrollTrigger: { trigger: "#family", start: "top 65%", end: "bottom 35%", scrub: true } })
    .fromTo(arc, { strokeDashoffset: 0 }, { strokeDashoffset: 565.49 * (1 - 5 / 60), ease: "none" }, 0)
    .to(
      clock,
      {
        v: 5,
        ease: "none",
        onUpdate: () => {
          const t = `${Math.round(clock.v)}:00`;
          if (readout.textContent !== t) readout.textContent = t;
        },
      },
      0,
    )
    .to(dots, { backgroundColor: accent, borderColor: accent, stagger: 0.25, ease: "none" }, 0.45)
    .to(".ring-wrap", { scale: 1.06, duration: 0.12, ease: "power2.out", yoyo: true, repeat: 3 });

  // ---------- Credits: each line comes up out of the dark as you scroll ----------
  gsap.utils.toArray(".credit").forEach((c) => {
    gsap.fromTo(
      c,
      { opacity: 0.08, y: 70, filter: "blur(8px)" },
      {
        opacity: 1,
        y: 0,
        filter: "blur(0px)",
        ease: "none",
        scrollTrigger: { trigger: c, start: "top 96%", end: "top 58%", scrub: true },
      },
    );
  });

  // ---------- Plans: the Plus list lights line by line ----------
  gsap.from(".plan-plus li", {
    opacity: 0,
    x: -26,
    duration: 0.9,
    ease: "expo.out",
    stagger: 0.12,
    scrollTrigger: { trigger: ".plan-plus", start: "top 72%", once: true },
  });

  // ---------- Reveals: big headings rise out of a mask, everything else lifts and sharpens ----------
  gsap.utils.toArray(".display.reveal").forEach((el) => {
    SplitText.create(el, {
      type: "lines",
      mask: "lines",
      autoSplit: true,
      onSplit(self) {
        roomBelow(self);
        return gsap.from(self.lines, {
          yPercent: 115,
          duration: 1.2,
          ease: "expo.out",
          stagger: 0.1,
          scrollTrigger: { trigger: el, start: "top 92%", once: true },
        });
      },
    });
  });
  gsap.utils.toArray(".reveal:not(.display)").forEach((el) => {
    gsap.from(el, {
      opacity: 0,
      y: 30,
      filter: "blur(6px)",
      duration: 0.9,
      ease: "expo.out",
      scrollTrigger: { trigger: el, start: "top 94%", once: true },
    });
  });

  // ---------- Chapter HUD, the lower-third card and the shader's act ----------
  const tc = $("[data-tc]");
  const hud = $(".hud");
  const chapterLabel = $("[data-chapter-label]");
  const hudBar = $(".hud-bar i");
  const clockText = (secs) => `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, "0")}`;
  let lastTc = "";
  ST.create({
    start: 0,
    end: "max",
    onUpdate: (s) => {
      const t = clockText(Math.round(s.progress * 90));
      if (t !== lastTc) {
        lastTc = t;
        tc.textContent = t;
      }
      hudBar.style.transform = `scaleX(${s.progress.toFixed(4)})`;
      shader?.setAct(s.progress);
    },
  });
  // The timecode waits until the hero's buttons have scrolled clear of it
  ST.create({
    trigger: ".hero",
    start: "bottom 70%",
    onEnter: () => hud.classList.add("on"),
    onLeaveBack: () => hud.classList.remove("on"),
  });

  const card = $(".chapter-card");
  const cardNo = $("[data-card-no]");
  const cardName = $("[data-card-name]");
  const chapters = [...document.querySelectorAll("[data-chapter]")];
  const names = [...new Set(chapters.map((s) => s.dataset.chapter))];
  let shownName = "";
  const showCard = (name) => {
    cardNo.textContent = String(names.indexOf(name) + 1).padStart(2, "0");
    cardName.textContent = name;
    gsap.killTweensOf(card);
    gsap
      .timeline()
      .fromTo(card, { opacity: 0, y: 18 }, { opacity: 1, y: 0, duration: 0.6, ease: "expo.out" })
      .to(card, { opacity: 0, y: -10, duration: 0.6, ease: "power2.in" }, "+=2");
  };
  chapters.forEach((sec) => {
    ST.create({
      trigger: sec,
      start: "top 55%",
      end: "bottom 55%",
      onToggle: (self) => {
        if (!self.isActive) return;
        const name = sec.dataset.chapter;
        chapterLabel.textContent = name;
        if (name !== shownName && shownName !== "") showCard(name);
        shownName = name;
      },
    });
  });

  // ---------- Letterbox: black bars slide in during the big scenes ----------
  const lb = $(".letterbox");
  const bars = new Set();
  [".dolly", ".mouse", ".pin"].forEach((sel) => {
    const el = $(sel);
    ST.create({
      trigger: el,
      start: "top 45%",
      end: "bottom 55%",
      onToggle: (self) => {
        self.isActive ? bars.add(sel) : bars.delete(sel);
        gsap.to(lb, { "--lb": bars.size ? 1 : 0, duration: 0.9, ease: "power3.out", overwrite: true });
      },
    });
  });

  // ---------- Your look: the page really flips to the other theme while you read about it ----------
  if (fine && window.JoyTheme) {
    ST.create({
      trigger: ".looks .lede",
      start: "top 62%",
      end: "bottom+=140 38%",
      onToggle: (self) => window.JoyTheme.preview(self.isActive),
    });
  }

  // ---------- Finale: the headline comes in from far away and the light warms ----------
  gsap.fromTo(
    ".final .display",
    { scale: 0.7, filter: "blur(10px)" },
    {
      scale: 1,
      filter: "blur(0px)",
      ease: "none",
      scrollTrigger: { trigger: ".final", start: "top 85%", end: "top 25%", scrub: true },
    },
  );
  ST.create({
    trigger: ".final",
    start: "top bottom",
    end: "bottom bottom",
    onUpdate: (s) => shader?.setWarm(s.progress),
  });
  const glint = gsap
    .timeline({ repeat: -1, repeatDelay: 2.4, paused: true })
    .fromTo(".sweep", { xPercent: -160 }, { xPercent: 260, duration: 1.6, ease: "power2.inOut" });
  ST.create({
    trigger: ".final .btn",
    start: "top 95%",
    end: "bottom 5%",
    onToggle: (self) => (self.isActive ? glint.play() : glint.pause()),
  });

  // ---------- Fine pointers: a glass cursor, labels, leaning buttons and a little scroll lean ----------
  if (fine) {
    document.documentElement.classList.add("has-cursor");
    const cursor = $(".cursor");
    [
      [".dolly-frame", "Look"],
      [".fan-card", "Watch"],
      [".mouse-art", "Play"],
    ].forEach(([sel, label]) => document.querySelectorAll(sel).forEach((el) => (el.dataset.cursor = label)));

    const cx = gsap.quickTo(cursor, "x", { duration: 0.45, ease: "power3" });
    const cy = gsap.quickTo(cursor, "y", { duration: 0.45, ease: "power3" });
    addEventListener("pointermove", (e) => {
      cursor.classList.add("on");
      cx(e.clientX);
      cy(e.clientY);
    }, { passive: true });
    document.addEventListener("pointerover", (e) => {
      const tag = e.target.closest("[data-cursor]");
      cursor.classList.toggle("has-label", Boolean(tag));
      if (tag) cursor.dataset.label = tag.dataset.cursor;
      cursor.classList.toggle("is-hot", !tag && Boolean(e.target.closest("a, button, summary")));
    });

    document.querySelectorAll(".btn-large").forEach((btn) => {
      const mx = gsap.quickTo(btn, "x", { duration: 0.7, ease: "elastic.out(1, 0.45)" });
      const my = gsap.quickTo(btn, "y", { duration: 0.7, ease: "elastic.out(1, 0.45)" });
      btn.addEventListener("pointermove", (e) => {
        const r = btn.getBoundingClientRect();
        mx((e.clientX - r.left - r.width / 2) * 0.22);
        my((e.clientY - r.top - r.height / 2) * 0.3);
      });
      btn.addEventListener("pointerleave", () => {
        mx(0);
        my(0);
      });
    });

    // Headlines lean a few degrees in the direction of a fast scroll, then settle
    const lean = gsap.utils.toArray(".display:not(.final .display), .mouse-title, .pin-title");
    const setLean = gsap.quickSetter(lean, "skewY", "deg");
    const leanState = { v: 0 };
    ST.create({
      onUpdate: (self) => {
        const v = gsap.utils.clamp(-4, 4, self.getVelocity() / -450);
        if (Math.abs(v) <= Math.abs(leanState.v)) return;
        leanState.v = v;
        setLean(v);
        gsap.to(leanState, { v: 0, duration: 0.8, ease: "power3", overwrite: true, onUpdate: () => setLean(leanState.v) });
      },
    });
  }

  // Measure everything once the layout has settled (the track is now sideways)
  requestAnimationFrame(() => ST.refresh());
}

window.JoyScenes = { init };
})();
