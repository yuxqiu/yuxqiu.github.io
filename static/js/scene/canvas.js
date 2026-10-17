// Draws this page's footer scene (scene.js) on the canvas inside .scene, and
// animates it unless the visitor prefers reduced motion. Loaded by base.html.

import { GROUND, H, layersFor, NARROW_MAX, NARROW_SCALE, paint, recolor, SCALE } from "./scene.js";

const box = document.querySelector(".scene");
if (box) {
  const cv = box.querySelector("canvas");
  const ctx = cv.getContext("2d");
  const footer = document.querySelector(".site-footer");
  const layers = layersFor(box.dataset.path || "/");
  const narrow = matchMedia(`(max-width: ${NARROW_MAX}px)`);

  // The footer's actual background, as #rrggbb. It is GROUND unless a
  // dark-mode extension recolored the page, in which case the scene follows.
  let ground = GROUND;
  let tint = null;
  const readGround = () => {
    const m = footer && getComputedStyle(footer).backgroundColor.match(/\d+/g);
    const now = m ? `#${m.slice(0, 3).map((v) => (+v).toString(16).padStart(2, "0")).join("")}` : GROUND;
    if (now === ground) return false;
    ground = now;
    tint = recolor(now);
    return true;
  };

  const fill = (x, y, w, h, c) => {
    ctx.fillStyle = tint ? tint(c) : c;
    ctx.fillRect(x, y, w, h);
  };
  let W = 0;
  let t = 0;
  const frame = () => {
    readGround();
    ctx.clearRect(0, 0, W, H);
    paint(fill, layers, W, t);
  };
  const size = () => {
    const s = narrow.matches ? NARROW_SCALE : SCALE;
    W = Math.ceil(box.clientWidth / s);
    cv.width = W;
    cv.height = H;
    cv.style.width = `${W * s}px`;
    cv.style.height = `${H * s}px`;
    frame();
  };
  cv.hidden = false;
  size();
  new ResizeObserver(size).observe(box);
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    // No animation loop to notice a recolor, so watch for one: extensions
    // mark <html> and inject stylesheets into <head>.
    new MutationObserver(() => readGround() && frame()).observe(document.documentElement, { attributes: true, childList: true, subtree: true });
  } else {
    setInterval(() => {
      if (document.hidden) return;
      t++;
      frame();
    }, 160);
  }
}
