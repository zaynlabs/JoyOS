(() => {
// The page's background light, drawn in one fragment shader. The canvas is
// opaque: the page colour is mixed in here, so every browser composites the
// same picture. It is dark glass with a light moving inside it: a teal key
// light, an amber fill and a thin horizontal streak, all kept dim enough that
// text always reads. It runs at reduced resolution and steps down if frames
// get slow. If WebGL isn't available the CSS glow in the hero is used.

const VERT = `
attribute vec2 a;
void main() { gl_Position = vec4(a, 0.0, 1.0); }
`;

const FRAG = `
precision mediump float;
uniform vec2 u_res;
uniform float u_t;
uniform vec2 u_m;
uniform float u_warm;
uniform float u_act;
uniform vec3 u_bg;
uniform float u_light;

float h(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }

float n(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(h(i), h(i + vec2(1.0, 0.0)), f.x),
             mix(h(i + vec2(0.0, 1.0)), h(i + vec2(1.0, 1.0)), f.x), f.y);
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  for (int i = 0; i < 4; i++) {
    v += a * n(p);
    p = p * 2.03 + vec2(1.7, 9.2);
    a *= 0.5;
  }
  return v;
}

void main() {
  vec2 uv = gl_FragCoord.xy / u_res;
  float asp = u_res.x / u_res.y;
  vec2 p = vec2((uv.x - 0.5) * asp, uv.y - 0.5);
  vec2 m = (u_m - 0.5) * 0.3;
  float t = u_t * 0.05;

  // Slow warp so the light feels like it moves through glass
  vec2 q = vec2(fbm(p * 1.4 + m + t), fbm(p * 1.4 - m - t + 4.7));
  vec2 w = p + (q - 0.5) * 0.55;

  // Key light travels across the frame as the page scrolls
  vec2 key = vec2(0.34 * asp - u_act * 0.9 * asp, 0.22 - u_act * 0.12) + m * 0.8;
  float k = exp(-dot(w - key, w - key) * 2.6);
  // Amber fill sits low on the opposite side
  vec2 fill = vec2(-0.3 * asp + u_act * 0.7 * asp, -0.3 + u_act * 0.2);
  float f = exp(-dot(w - fill, w - fill) * 3.2);
  // A thin horizontal streak crossing the key light, like a lens
  float streak = exp(-abs(p.y - key.y + 0.02) * 70.0) * exp(-abs(p.x - key.x) * 1.7);

  vec3 teal = vec3(0.24, 0.72, 0.8);
  vec3 amber = mix(vec3(0.9, 0.52, 0.28), vec3(0.98, 0.76, 0.5), u_warm);
  // Teal early in the film, amber as it warms toward the end
  float mixAmt = clamp(u_act * 1.15 + u_warm * 0.6, 0.0, 1.0);
  vec3 keyCol = mix(teal, amber, mixAmt);
  vec3 fillCol = mix(amber, teal, 0.45 * (1.0 - u_warm));

  float cap = mix(0.34, 0.26, u_light);
  vec3 col = u_bg;
  col = mix(col, keyCol, clamp(k * 0.75 + q.x * 0.06, 0.0, 1.0) * cap);
  col = mix(col, fillCol, clamp(f * 0.7, 0.0, 1.0) * cap * (0.7 + 0.5 * u_warm));
  // Finale: the whole frame warms
  col = mix(col, amber, u_warm * 0.2 * (1.0 - u_light * 0.4));
  col += vec3(0.98, 0.74, 0.44) * u_warm * 0.2 * exp(-length(p - vec2(0.0, -0.3)) * 1.7) * (1.0 - u_light * 0.7);
  col += keyCol * streak * mix(0.16, 0.05, u_light);

  // Darken the edges so the middle of the frame glows
  float vig = smoothstep(1.35, 0.25, length(uv - 0.5) * 1.15);
  col = mix(u_bg, col, mix(0.55, 1.0, vig));

  // Tiny noise keeps the dark gradients from banding
  col += (h(gl_FragCoord.xy + u_t) - 0.5) / 255.0;
  gl_FragColor = vec4(col, 1.0);
}
`;

function pageColour() {
  // Computed colour of the body: "rgb(r, g, b)", already resolved for the theme
  const m = getComputedStyle(document.body).backgroundColor.match(/[\d.]+/g);
  if (!m || m.length < 3) return [0.043, 0.043, 0.051];
  return [m[0] / 255, m[1] / 255, m[2] / 255];
}

function initLight(canvas) {
  const gl = canvas.getContext("webgl", {
    alpha: false,
    antialias: false,
    depth: false,
    powerPreference: "low-power",
  });
  if (!gl) return null;

  const compile = (type, src) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, src);
    gl.compileShader(shader);
    return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
  };
  const vs = compile(gl.VERTEX_SHADER, VERT);
  const fs = compile(gl.FRAGMENT_SHADER, FRAG);
  if (!vs || !fs) return null;

  const prog = gl.createProgram();
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return null;
  gl.useProgram(prog);

  // One oversized triangle covers the screen
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  const loc = gl.getAttribLocation(prog, "a");
  gl.enableVertexAttribArray(loc);
  gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

  const U = {};
  for (const name of ["u_res", "u_t", "u_m", "u_warm", "u_act", "u_bg", "u_light"]) {
    U[name] = gl.getUniformLocation(prog, name);
  }

  const small = matchMedia("(max-width: 760px)").matches;
  let scale = small ? 0.35 : 0.5;
  const minScale = 0.22;
  let width = 0;
  let height = 0;

  const resize = () => {
    width = Math.max(1, Math.round(innerWidth * scale));
    height = Math.max(1, Math.round(innerHeight * scale));
    canvas.width = width;
    canvas.height = height;
    gl.viewport(0, 0, width, height);
  };
  resize();
  addEventListener("resize", resize);

  // The page colour changes with the theme; read it again when that happens
  let bg = pageColour();
  const readTheme = () => {
    bg = pageColour();
  };
  new MutationObserver(readTheme).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", readTheme);

  const pointer = [0.5, 0.5];
  const eased = [0.5, 0.5];
  let warm = 0;
  let warmTarget = 0;
  let act = 0;
  let actTarget = 0;
  let light = 0;
  const start = performance.now();
  let last = start;
  let slow = 0;
  let shown = false;

  addEventListener(
    "pointermove",
    (e) => {
      pointer[0] = e.clientX / innerWidth;
      pointer[1] = 1 - e.clientY / innerHeight;
    },
    { passive: true },
  );

  const draw = (now) => {
    requestAnimationFrame(draw);
    if (document.hidden) {
      last = now;
      return;
    }

    // Step down the resolution if frames are consistently slow
    const dt = now - last;
    last = now;
    if (dt > 26) slow++;
    else slow = Math.max(0, slow - 1);
    if (slow > 45 && scale > minScale) {
      scale = Math.max(minScale, scale * 0.75);
      slow = 0;
      resize();
    }

    eased[0] += (pointer[0] - eased[0]) * 0.04;
    eased[1] += (pointer[1] - eased[1]) * 0.04;
    warm += (warmTarget - warm) * 0.05;
    act += (actTarget - act) * 0.06;
    light += ((bg[0] + bg[1] + bg[2]) / 3 > 0.5 ? 1 - light : -light) * 0.1;

    gl.uniform2f(U.u_res, width, height);
    gl.uniform1f(U.u_t, (now - start) / 1000);
    gl.uniform2f(U.u_m, eased[0], eased[1]);
    gl.uniform1f(U.u_warm, warm);
    gl.uniform1f(U.u_act, act);
    gl.uniform3f(U.u_bg, bg[0], bg[1], bg[2]);
    gl.uniform1f(U.u_light, light);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    if (!shown) {
      shown = true;
      canvas.classList.add("on");
      document.documentElement.classList.add("has-gl");
    }
  };
  requestAnimationFrame(draw);

  return {
    // 0 is the normal glow; 1 is the warm finale
    setWarm(v) {
      warmTarget = Math.min(1, Math.max(0, v));
    },
    // 0 at the top of the page, 1 at the bottom: moves the key light
    setAct(v) {
      actTarget = Math.min(1, Math.max(0, v));
    },
  };
}

window.JoyShader = { initLight };
})();
