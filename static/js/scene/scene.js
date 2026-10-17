// Which footer scene each page gets, as layers ready to paint.
//
// A few pages have a pinned scene (PINNED); every other page composes its own
// from the kit, seeded by its path, so a page keeps its scene and different
// pages show different small things.
//
// A scene is an ordered list of layers, bottom to top. Each layer is
//   { id, w, at, tile, paint(fill, W, x, t) }
// where w is its width in scene pixels; tile layers repeat every w pixels
// from x = 0; other layers sit with their left edge at `at` (0..1) of the
// leftover width, x = round(at × (W − w)), which is how CSS places a
// background at `at × 100%`. paint draws frame t on a scene W pixels wide via
// fill(x, y, w, h, "#rrggbb"). At t = 0 a non-tile layer paints only inside
// [x, x + w), so each one can be cut out as its own no-JS image.
//
// Used by canvas.js (the live scene) and scripts/scene/render.mjs (the no-JS
// fallback). No DOM here.

import { BASES, GROUND, H, hash, ITEMS, PERIOD, pen, rng } from "./kit.js";

export { GROUND, H };
export const SCALE = 3; // CSS px per scene pixel
export const NARROW_SCALE = 2; // … at viewport widths up to NARROW_MAX
export const NARROW_MAX = 560;

// Pages whose scene is fixed by hand rather than drawn from the seed, keyed by
// path. Changing the kit's choices or the seeding never moves these.
const PINNED = {
  // A cat asleep on a stack of books in the meadow.
  "/": { base: "meadow", items: [["catBooks", 0.2], ["flowers", 0.5], ["tree", 0.82], ["walker", 0.55]] },
  // A meadow with a tree and a few flowers.
  "/publications/": { base: "meadow", items: [["tree", 0.06], ["flowers", 0.94], ["walker", 0.19]] },
};

// What visitors without JS see on pages that aren't pinned.
const FALLBACK = { base: "lake", items: [["walker", 0.3]] };

function seeded(path) {
  const r = rng(hash(path));
  const base = r() < 0.55 ? "lake" : "meadow";
  const pool = base === "lake" ? ["reeds", "reeds", "dock", "catBooks", "tree"] : ["flowers", "flowers", "tree", "catBooks", "butterfly", "kite"];
  const slots = [0.06, 0.3, 0.52, 0.74, 0.94]
    .map((f) => [f, r()])
    .sort((a, b) => a[1] - b[1])
    .slice(0, 2 + ((r() * 2) | 0))
    .map(([f]) => f)
    .sort((a, b) => a - b);
  const used = new Set();
  const items = slots.map((f) => {
    let it;
    do it = pool[(r() * pool.length) | 0];
    while (used.has(it) && it !== "reeds" && it !== "flowers");
    used.add(it);
    return [it, f];
  });
  if (base === "lake" && r() < 0.4) items.push(["boat", rng(hash(`${path}#boat`))()]);
  if (!used.has("kite") && r() < 0.8) items.push(["walker", 0.15 + r() * 0.6]);
  return { base, items };
}

// One PERIOD of each base, built once from a fixed seed.
const baseLayouts = {};
const baseLayout = (name) => (baseLayouts[name] ??= BASES[name].build(rng(hash(name))));

// `key` names the scene; an item's random details depend only on it and the
// item's slot.
function layersOf({ base, items }, key) {
  const layout = baseLayout(base);
  return [
    { id: `base-${base}`, w: PERIOD, at: 0, tile: true, paint: (fill, W, x, t) => BASES[base].draw(pen(fill, W), W, layout, t) },
    ...items.map(([name, at], i) => {
      const item = ITEMS[name];
      const o = item.build(rng(hash(`${key}#${i}`)));
      return {
        id: o === null ? name : `${name}-${hash(`${key}#${i}`).toString(36)}`,
        w: item.w,
        at,
        tile: false,
        paint: (fill, W, x, t) => item.draw(pen(fill, W), x, o, t, W),
      };
    }),
  ];
}

// The layers of the scene at `path`.
export function layersFor(path) {
  return layersOf(PINNED[path] ?? seeded(path), path);
}

// The scenes that get a no-JS fallback: path null is every page that isn't
// pinned; then each pinned page.
export function fallbacks() {
  return [{ path: null, layers: layersOf(FALLBACK, "fallback") }, ...Object.keys(PINNED).map((path) => ({ path, layers: layersFor(path) }))];
}

// Paints frame t of `layers` on a scene W pixels wide.
export function paint(fill, layers, W, t) {
  for (const L of layers) L.paint(fill, W, L.tile ? 0 : Math.round(L.at * (W - L.w)), t);
}

// Dark-mode extensions (Dark Reader and the like) recolor a page's CSS but
// not canvas pixels, which leaves a light scene above a darkened footer.
// Given the footer's actual color ("#rrggbb"; GROUND unless something
// recolored it), returns a function mapping the scene's colors to match: null
// when no change is needed, otherwise one that flips lightness around the new
// ground, so GROUND lands exactly on it and dark details turn light.
export function recolor(ground) {
  const [gh, gs, gl] = toHsl(ground);
  if (ground === GROUND || gl >= 0.5) return null;
  const sandL = toHsl(GROUND)[2];
  const k = (0.8 - gl) / (sandL - 0.2); // the darkest inks end up near 80% lightness
  const memo = new Map([[GROUND, ground]]);
  return (c) => {
    if (!memo.has(c)) {
      const [h, s, l] = toHsl(c);
      const L = Math.min(1, Math.max(0, gl + (sandL - l) * k));
      // Near-neutral warm grays take the new ground's tint, so hills and
      // water match the footer; real colors keep their own hue.
      memo.set(c, chroma(c) < 0.095 ? fromHsl(gh, gs, L) : fromHsl(h, s * 0.85, L));
    }
    return memo.get(c);
  };
}

// Distance from gray, 0..1: the kit's warm grays measure up to 0.086, its
// lightest sage 0.102, wood and the accents more.
function chroma(hex) {
  const v = [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16) / 255);
  return Math.max(...v) - Math.min(...v);
}

function toHsl(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16) / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = d / (1 - Math.abs(2 * l - 1));
  const h = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
}

function fromHsl(h, s, l) {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return `#${[r, g, b].map((v) => Math.round((v + m) * 255).toString(16).padStart(2, "0")).join("")}`;
}
