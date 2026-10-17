// Tests for the footer scene's layer model (static/js/scene/scene.js).
// Run: node --test scripts/scene/

import assert from "node:assert/strict";
import { test } from "node:test";
import { fallbacks, H, layersFor, paint } from "../../static/js/scene/scene.js";
import { raster } from "./raster.mjs";

// Composite each layer's frame-0 cut-out the way scene.css stacks the PNGs.
function composite(layers, W) {
  const out = Buffer.alloc(W * H * 4);
  for (const L of layers) {
    const img = raster(L.w, H, (fill) => L.paint(fill, L.w, 0, 0));
    const starts = L.tile ? Array.from({ length: Math.ceil(W / L.w) }, (_, k) => k * L.w) : [Math.round(L.at * (W - L.w))];
    for (const x0 of starts) {
      for (let y = 0; y < H; y++) {
        for (let i = 0; i < L.w; i++) {
          const x = x0 + i;
          const src = (y * L.w + i) * 4;
          if (x < 0 || x >= W || img[src + 3] === 0) continue;
          img.copy(out, (y * W + x) * 4, src, src + 4);
        }
      }
    }
  }
  return out;
}

test("each no-JS fallback matches its live scene at frame 0", () => {
  for (const { path, layers } of fallbacks()) {
    for (const W of [187, 427, 640, 1100]) {
      const live = raster(W, H, (fill) => paint(fill, layers, W, 0));
      assert.ok(composite(layers, W).equals(live), `${path ?? "fallback"} at W=${W}`);
    }
  }
});

test("a page's scene is the same every time, and pages differ", () => {
  const paths = ["/blog/", "/misc/", "/blog/concurrency/", "/tags/cpp/", "/blog/attacks-on-rsa/"];
  const frames = paths.map((p) => raster(427, H, (fill) => paint(fill, layersFor(p), 427, 7)));
  paths.forEach((p, i) => assert.ok(frames[i].equals(raster(427, H, (fill) => paint(fill, layersFor(p), 427, 7))), p));
  assert.equal(new Set(frames.map((f) => f.toString("base64"))).size, paths.length);
});
