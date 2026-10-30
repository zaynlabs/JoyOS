# JoyOS landing page

A static site. No build step, no tracking. It reads fine without JavaScript, and the motion layer is added on top.

## Before you publish

1. Check the download link. The site uses `https://github.com/ZaynLabs/JoyOS/releases/latest`. It appears in the header, hero, plans, and finale.
2. Set the Plus price in the Plans section (it says "Price set at launch.").
3. Set the full `og:image` address so link previews work. Social apps need an absolute `https://` address.

## Hosting

Upload this folder to any static host:

- Cloudflare Pages: connect the repo, set the build output directory to `site`, no build command.
- GitHub Pages: publish the `site` folder.

To preview: open `site/index.html` directly in a browser. It works from disk. The server options below are optional.

## Structure

- `index.html`: all copy and structure. Scenes are marked with `data-chapter`, which drives the timecode.
- `styles.css`: tokens (same as `assets/css/joy.css`), layout, and every scene's state.
- `js/main.js`: entry point. Starts the demo, clips, theme switch, greeting, and phone download bar right away. Loads the motion libraries after the first paint, only if the visitor allows motion.
- `js/scenes.js`: GSAP scenes: the headline, push-in, mouse drawing, pinned sections, horizontal track, app mock, timer ring, timecode, cursor, and finale.
- `js/shader.js`: WebGL background light. Lowers its resolution when frames are slow. Hidden if WebGL is unavailable.
- `js/demo.js`: the working mini-JoyOS. Keyboard, touch, and the Gamepad API. Nothing is launched.
- `vendor/`: GSAP 3.15.0 (with ScrollTrigger and SplitText), Lenis 1.3.27. Copied from npm without changes. GSAP ships under its standard no-charge license (see the GSAP package); Lenis is MIT (`vendor/LICENSE-lenis.txt`).
- `assets/`: WebP stills, two H.264 clips, and Latin-subset WOFF fonts. Made from `assets/marketing/` and `assets/fonts/`.

## Motion settings

- Reduced motion: no libraries are loaded, no pinning, no shader, no grain. The page is the static layout.
- Fine pointers (mouse, trackpad): smooth scrolling and a glass cursor.
- Touch: native scrolling. The store track stacks vertically on phones.
- Save-data: the shader is skipped.
