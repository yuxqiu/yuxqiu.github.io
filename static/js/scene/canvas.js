// Draws this page's footer scene (scene.js) on the canvas inside .scene, and
// animates it unless the visitor prefers reduced motion. Loaded by base.html.

import { H, layersFor, NARROW_MAX, NARROW_SCALE, paint, SCALE } from "./scene.js";

const box = document.querySelector(".scene");
if (box) {
  const cv = box.querySelector("canvas");
  const ctx = cv.getContext("2d");
  const layers = layersFor(box.dataset.path || "/");
  const narrow = matchMedia(`(max-width: ${NARROW_MAX}px)`);
  const fill = (x, y, w, h, c) => {
    ctx.fillStyle = c;
    ctx.fillRect(x, y, w, h);
  };
  let W = 0;
  let t = 0;
  const frame = () => {
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
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
    setInterval(() => {
      if (document.hidden) return;
      t++;
      frame();
    }, 160);
  }
}
