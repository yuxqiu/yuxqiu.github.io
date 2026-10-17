// The footer scene's kit: what everything looks like and how it moves.
// Bases are backdrops that repeat every PERIOD pixels; items are things placed
// on them (the walker is one too). Which of these a page gets, and where, is
// decided in scene.js.
//
// Coordinates are in scene pixels. Colors must be #rrggbb: the build script
// (scripts/scene/render.mjs) parses them by hand.

export const H = 40;
export const PERIOD = 480;
const HOR = 12; // lake horizon
const SHORE = 28; // first row of the lake's bank
const MEADOW = 26; // first row of meadow ground
const FEET = H - 4; // row people and items stand on
const TAU = Math.PI * 2;

// Kami palette plus a few muted naturals.
const K = {
  ink: "#1b365d",
  ivory: "#faf9f5",
  sand: "#e8e6dc", // --warm-sand: the ground of both bases and the footer background
  w200: "#d8d2c4",
  w300: "#c4bdae",
  w400: "#a39d8e",
  w700: "#524e45",
  w800: "#3d3a33",
  w900: "#2d2a24",
  mtnFar: "#e3dfd3",
  mtn: "#cdc6b7",
  water: "#dfdbd0",
  water2: "#d5cfc2",
  rip: "#ebe8df",
  ripMid: "#e5e2d8",
  ripDark: "#cfc9bb",
  ripDark2: "#c6bfb1",
  sage: "#a9a98c",
  sageDark: "#8e8f73",
  sageLight: "#cfcdb5",
  meadowEdge: "#d6d3c1",
  ochre: "#c9a35b",
  ochreGlow: "#dfbd7c",
  rust: "#b5714f",
  ginger: "#c98d58",
  skin: "#d9b48f",
  wood: "#9a8268",
  woodDark: "#75624f",
};

// ---------- sprites ----------

const HEAD = ["..hhh..", ".hhhhh.", "hhhhhhh", "..sss..", "..ssk..", "..jjj.."];
const WALK = [
  [...HEAD, ".jjjjj.", ".jjjjj.", ".sjjjs.", "..ppp..", ".p...p.", ".k...k."], // stride
  [...HEAD, ".jjjjj.", ".jjjjj.", "..jjjs.", "..ppp..", "...p...", "...kk.."], // passing
  [...HEAD, ".jjjjj.", ".jjjjj.", ".sjjjs.", "..ppp..", ".p...p.", ".k...k."], // stride
  [...HEAD, ".jjjjj.", ".jjjjj.", ".sjjj..", "..ppp..", "...p...", "...kk.."], // passing
];
const KITER = ["..hhh..", ".hhhhh.", "hhhhhhh", "..sss..", "..ssk.s", "..jjjj.", ".jjjjj.", ".jjjjj.", ".sjjj..", "..ppp..", "..p.p..", "..k.k.."];
const PERSON = { h: K.w300, s: K.skin, k: K.w900, j: K.ink, p: K.w700 };
const PERSON_Y = FEET - WALK[0].length + 1;

// Curled-up cat, head on the left; the tail is drawn separately so it can flick.
const CAT = [".o..o.......", ".oooo.ooo...", "occcoocccoo.", "ococcccccco.", "occcccccccco", "occcccccccco", ".oooooooooo."];
const CAT_TAIL = [
  [[12, 5], [12, 6], [11, 6]],
  [[12, 3], [12, 4], [13, 2]],
];
const BOOKS = ["..aaaaaaa..", "..appppppa.", ".bbbbbbbbb.", ".bpppppppb.", "ccccccccccc", "cpppppppppc"];
const ZZ = ["ooo", ".o.", "ooo"];
const BOAT = ["......hhh.......", ".....hhhhh......", "......sss.......", "......jjj.......", "bbbbbbbbbbbbbbbb", ".dbbbbbbbbbbbbd.", "..dddddddddddd.."];
const KITE = ["..a..", ".aab.", "aaabb", ".abb.", "..b.."];

// ---------- helpers ----------

export function rng(seed) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function hash(str) {
  let h = 2166136261;
  for (const ch of str) h = Math.imul(h ^ ch.charCodeAt(0), 16777619);
  return h >>> 0;
}

const mod = (a, n) => ((a % n) + n) % n;

// Left edge of something w wide that started at x and has moved dx to the
// right, wrapping around a scene W wide (fully off one side before it
// reappears on the other). With dx = 0 it is exactly x.
const travel = (x, w, W, dx) => Math.round(mod(x + dx + w, W + 2 * w)) - w;

// Wraps fill(x, y, w, h, color) with rounding, clipping to [0, W), sprites and lines.
export function pen(fill, W) {
  const R = (x, y, w, h, c) => {
    x = Math.round(x);
    if (c && w > 0 && h > 0 && x < W && x + w > 0) fill(x, Math.round(y), w, h, c);
  };
  const P = (x, y, c) => R(x, y, 1, 1, c);
  const spr = (x, y, rows, pal) =>
    rows.forEach((row, j) => {
      for (let i = 0; i < row.length; i++) if (row[i] !== ".") P(x + i, y + j, pal[row[i]]);
    });
  const line = (x0, y0, x1, y1, c, step = 1) => {
    const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0));
    for (let i = 0; i <= n; i += step) P(x0 + ((x1 - x0) * i) / n, y0 + ((y1 - y0) * i) / n, c);
  };
  return { R, P, spr, line };
}

// ---------- bases ----------
// build(r) lays out one PERIOD from a random source; draw(d, W, L, t) repeats
// it across the width. Skies are transparent.

export const BASES = {
  lake: {
    build(r) {
      const a = r() * TAU;
      const b = r() * TAU;
      // Whole cycles per PERIOD so the skyline tiles.
      const mt = Array.from({ length: PERIOD }, (_, x) => {
        const f = x / PERIOD;
        const p = (TAU * x) / PERIOD;
        if (f > 0.55) {
          const taper = Math.min(1, (f - 0.55) * 12, (1 - f) * 14);
          return Math.max(0, Math.round((3 + 2.5 * Math.sin(3 * p + a) + 1.6 * Math.sin(10 * p + b)) * taper));
        }
        if (f < 0.3) return Math.max(0, Math.round((1.2 + Math.sin(14 * p + b)) * Math.min(1, f * 20, (0.3 - f) * 20)));
        return 0;
      });
      const ripples = Array.from({ length: 64 }, () => ({
        x: (r() * (PERIOD - 6)) | 0,
        y: HOR + 2 + ((r() * (SHORE - HOR - 4)) | 0),
        l: 2 + ((r() * 4) | 0),
        ph: (r() * 60) | 0,
        p: 36 + ((r() * 30) | 0),
      }));
      const pebbles = Array.from({ length: 40 }, () => ({ x: (r() * PERIOD) | 0, y: SHORE + 2 + ((r() * (H - SHORE - 2)) | 0) }));
      return { mt, ripples, pebbles };
    },
    draw(d, W, L, t) {
      for (let x = 0; x < W; x++) {
        const px = x % PERIOD;
        const h = L.mt[px];
        if (h) d.R(x, HOR - h, 1, h, px / PERIOD > 0.5 ? K.mtn : K.mtnFar);
      }
      for (let y = HOR; y < SHORE; y++) d.R(0, y, W, 1, y > HOR + 8 ? K.water2 : K.water);
      for (let x = 1; x < W; x += 2) d.P(x, HOR + 8, K.water); // soften the band edge
      d.R(0, SHORE, W, H - SHORE, K.sand);
      for (let x = 0; x < W; x++) if (x % 3 !== 1) d.P(x, SHORE, K.w300); // waterline
      for (let k = 0; k < W; k += PERIOD) {
        // Ripples fade in and out in place: a light line over a darker one.
        for (const q of L.ripples) {
          const n = (t + q.ph) % q.p;
          if (n >= 16) continue;
          const soft = n < 3 || n > 12;
          const deep = q.y > HOR + 8;
          d.R(k + q.x, q.y, q.l, 1, soft ? K.ripMid : K.rip);
          d.R(k + q.x + 1, q.y + 1, q.l - 1, 1, deep ? (soft ? K.water2 : K.ripDark2) : soft ? K.water : K.ripDark);
        }
        for (const p of L.pebbles) d.P(k + p.x, p.y, K.w200);
      }
    },
  },
  meadow: {
    build(r) {
      const a = r() * TAU;
      const b = r() * TAU;
      const hills = Array.from({ length: PERIOD }, (_, x) => {
        const p = (TAU * x) / PERIOD;
        return Math.max(0, Math.round(5 + 3 * Math.sin(4 * p + a) + 1.5 * Math.sin(11 * p + b)));
      });
      const tufts = Array.from({ length: 26 }, () => ({ x: 1 + ((r() * (PERIOD - 2)) | 0), y: MEADOW + 2 + ((r() * (H - MEADOW - 3)) | 0), ph: r() * TAU }));
      return { hills, tufts };
    },
    draw(d, W, L, t) {
      for (let x = 0; x < W; x++) {
        const h = L.hills[x % PERIOD];
        d.R(x, MEADOW - h, 1, h, K.mtnFar);
      }
      d.R(0, MEADOW, W, H - MEADOW, K.sand);
      for (let x = 0; x < W; x++) if (x % 2) d.P(x, MEADOW, K.meadowEdge);
      for (let k = 0; k < W; k += PERIOD) {
        for (const q of L.tufts) {
          const s = Math.sin(t * 0.08 + q.ph) > 0.6 ? 1 : 0;
          const x = k + q.x;
          d.P(x - 1, q.y, K.sage);
          d.P(x, q.y, K.sage);
          d.P(x + 1, q.y, K.sage);
          d.P(x + s, q.y - 1, K.sageDark);
          d.P(x - 1 + s, q.y - 1, K.sage);
        }
      }
    },
  },
};

// ---------- items ----------
// w is the width an item occupies. build(r) picks its own random details
// (null if it has none); draw(d, x, o, t, W) paints it at frame t with its
// left edge placed at x. At t = 0 nothing may be drawn outside [x, x + w):
// that frame is also cut out as the item's no-JS image.

export const ITEMS = {
  walker: {
    w: 7,
    build: () => null,
    // Strolls right, wrapping around; every WALK ticks he stops for PAUSE
    // ticks to look at the view.
    draw(d, x, o, t, W) {
      const WALK_T = 220;
      const PAUSE = 30;
      const lap = Math.floor(t / (WALK_T + PAUSE));
      const into = t % (WALK_T + PAUSE);
      const dist = (lap * WALK_T + Math.min(into, WALK_T)) * 0.5;
      const frame = into >= WALK_T ? 1 : Math.floor(into / 3) % 4;
      d.spr(travel(x, 7, W, dist), PERSON_Y, WALK[frame], PERSON);
    },
  },
  reeds: {
    w: 9,
    build: (r) => Array.from({ length: 5 + ((r() * 3) | 0) }, () => ({ dx: (r() * 8) | 0, h: 4 + ((r() * 6) | 0), tail: r() < 0.4, ph: r() * TAU })),
    draw(d, x, o, t) {
      for (const q of o) {
        const sway = Math.round(Math.sin(t * 0.07 + q.ph) * 0.9);
        for (let k = 0; k < q.h; k++) d.P(x + q.dx + (k > q.h * 0.55 ? sway : 0), SHORE + 1 - k, k % 3 ? K.sage : K.sageDark);
        if (q.tail) d.R(x + q.dx + sway, SHORE - q.h, 1, 2, K.woodDark);
      }
    },
  },
  dock: {
    w: 24,
    build: () => null,
    draw(d, x, o, t) {
      d.R(x, SHORE - 3, 24, 1, K.wood);
      d.R(x, SHORE - 2, 24, 1, K.woodDark);
      for (const px of [x + 2, x + 11, x + 20]) d.R(px, SHORE - 1, 1, 3, K.woodDark);
      // Lantern on a post at the end; it flickers now and then.
      const lit = (t >> 2) % 9 !== 0;
      d.R(x + 22, SHORE - 13, 1, 10, K.w800);
      d.P(x + 21, SHORE - 13, K.w800);
      d.R(x + 19, SHORE - 12, 3, 1, K.w800);
      d.R(x + 19, SHORE - 11, 3, 2, lit ? K.ochre : K.ochreGlow);
      d.R(x + 19, SHORE - 9, 3, 1, K.w800);
    },
  },
  catBooks: {
    w: 14,
    // A fixed phase, so frame 0 (the no-JS image) shows the cat at rest.
    build: () => ({ ph: 30 }),
    draw(d, x, o, t) {
      d.spr(x + 1, FEET - 5, BOOKS, { a: K.ink, b: K.rust, c: K.sageDark, p: K.ivory });
      const cy = FEET - 11;
      d.spr(x, cy, CAT, { o: K.w800, c: K.ginger });
      const flick = (t + o.ph) % 50 < 4 ? 1 : 0;
      for (const [tx, ty] of CAT_TAIL[flick]) d.P(x + tx, cy + ty, K.w800);
      const n = (t + o.ph) % 70; // a small z drifts up now and then
      if (n < 24) d.spr(x + 1 - (n >> 3), FEET - 15 - (n >> 2), ZZ, { o: n < 16 ? K.w400 : K.w300 });
    },
  },
  tree: {
    w: 15,
    build: (r) => ({ r: 5 + ((r() * 3) | 0) }),
    draw(d, x, o) {
      const cx = x + 7;
      const rr = o.r;
      const cy = FEET - 7 - rr;
      d.R(cx - 1, FEET - 7, 2, 8, K.woodDark);
      for (let dy = -rr; dy <= rr; dy++) {
        for (let dx = -rr; dx <= rr; dx++) {
          if (dx * dx + dy * dy > rr * rr + rr * 0.5) continue;
          d.P(cx + dx, cy + dy, dx + dy > rr * 0.5 && (dx + dy) % 2 === 0 ? K.sageDark : dx + dy < -rr * 0.6 ? K.sageLight : K.sage);
        }
      }
    },
  },
  flowers: {
    w: 17,
    build: (r) => Array.from({ length: 5 }, () => ({ dx: (r() * 16) | 0, h: 2 + ((r() * 3) | 0), c: [K.ochre, K.rust, K.ochre, K.ivory][(r() * 4) | 0], ph: r() * TAU })),
    draw(d, x, o, t) {
      for (const q of o) {
        const s = Math.sin(t * 0.06 + q.ph) > 0.7 ? 1 : 0;
        for (let k = 0; k < q.h; k++) d.P(x + q.dx + (k === q.h - 1 ? s : 0), FEET - k, K.sage);
        d.P(x + q.dx + s, FEET - q.h, q.c);
      }
    },
  },
  boat: {
    w: 16,
    build: () => null,
    // Crosses the whole lake very slowly, starting at x.
    draw(d, x, o, t, W) {
      const bx = travel(x, 16, W, t * 0.12);
      const by = HOR + 4;
      d.spr(bx, by, BOAT, { h: K.w300, s: K.skin, j: K.ink, b: K.wood, d: K.woodDark });
      const f = (t >> 3) % 2;
      d.line(bx + 8, by + 4, bx + (f ? 13 : 12), by + (f ? 7 : 8), K.woodDark);
      d.R(bx + 2, by + 7, 12, 1, K.ripMid);
      if (f) d.R(bx - 3, by + 6, 3, 1, K.rip);
    },
  },
  kite: {
    w: 43,
    build: () => null,
    // Someone standing still, flying a kite that bobs in the breeze.
    draw(d, x, o, t) {
      d.spr(x, PERSON_Y, KITER, PERSON);
      const kx = x + 34 + Math.round(Math.sin(t * 0.05) * 3);
      const ky = 2 + Math.round(Math.sin(t * 0.09) * 1.5);
      d.line(x + 6, PERSON_Y + 4, kx + 2, ky + 4, K.w400, 2);
      d.spr(kx, ky, KITE, { a: K.ink, b: K.rust });
      for (let i = 1; i <= 4; i++) d.R(kx + 2 - i + Math.round(Math.sin(t * 0.2 + i)), ky + 4 + i * 2, 2, 1, i % 2 ? K.rust : K.ochre);
    },
  },
  butterfly: {
    w: 45,
    build: (r) => ({ ph: r() * TAU }),
    draw(d, x, o, t) {
      const bx = x + 22 + Math.round(Math.sin(t * 0.04 + o.ph) * 22);
      const by = 14 + Math.round(Math.sin(t * 0.11 + o.ph) * 3);
      if ((t >> 1) % 2) {
        d.P(bx - 1, by, K.rust);
        d.P(bx + 1, by, K.rust);
      } else d.P(bx, by, K.rust);
      d.P(bx, by + 1, K.w800);
    },
  },
};
