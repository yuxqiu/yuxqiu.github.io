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

import { BASES, H, hash, ITEMS, PERIOD, pen, rng } from "./kit.js";

export { H };
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
