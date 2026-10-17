// Paints into an in-memory RGBA buffer through the same fill(x, y, w, h,
// "#rrggbb") callback the canvas uses. Unpainted pixels stay transparent.

export function raster(W, H, draw) {
  const rgba = Buffer.alloc(W * H * 4);
  draw((x, y, w, h, c) => {
    const px = [1, 3, 5].map((i) => Number.parseInt(c.slice(i, i + 2), 16)).concat(255);
    for (let j = Math.max(0, y); j < Math.min(H, y + h); j++) {
      for (let i = Math.max(0, x); i < Math.min(W, x + w); i++) rgba.set(px, (j * W + i) * 4);
    }
  });
  return rgba;
}
