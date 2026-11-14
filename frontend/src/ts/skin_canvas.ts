export type SkinViewMode = "front" | "back" | "atlas";

function rgbaToImageCanvas(rgba: number[]): HTMLCanvasElement | null {
  if (rgba.length !== 64 * 64 * 4) return null;
  const off = document.createElement("canvas");
  off.width = 64;
  off.height = 64;
  const octx = off.getContext("2d");
  if (!octx) return null;
  const imgData = octx.createImageData(64, 64);
  for (let i = 0; i < rgba.length; i += 1) {
    imgData.data[i] = rgba[i] ?? 0;
  }
  octx.putImageData(imgData, 0, 0);
  return off;
}

export function drawTopbarHead(canvas: HTMLCanvasElement, rgba: number[]): void {
  const ctx = canvas.getContext("2d");
  const src = rgbaToImageCanvas(rgba);
  if (!ctx || !src) return;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(src, 8, 8, 8, 8, 0, 0, canvas.width, canvas.height);
  ctx.drawImage(src, 40, 8, 8, 8, 0, 0, canvas.width, canvas.height);
}

export function drawSkinPreview(
  canvas: HTMLCanvasElement,
  rgba: number[],
  mode: SkinViewMode,
  isSlim = true
): void {
  const ctx = canvas.getContext("2d");
  const src = rgbaToImageCanvas(rgba);
  if (!ctx || !src) return;

  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.imageSmoothingEnabled = false;

  if (mode === "atlas") {
    const pad = 14;
    const side = Math.min(w - pad * 2, h - pad * 2);
    const ox = Math.floor((w - side) / 2);
    const oy = Math.floor((h - side) / 2);
    const step = side / 16;
    for (let ry = 0; ry < 16; ry += 1) {
      for (let rx = 0; rx < 16; rx += 1) {
        ctx.fillStyle = (rx + ry) % 2 === 0 ? "rgba(255,255,255,0.04)" : "rgba(0,0,0,0.06)";
        ctx.fillRect(ox + rx * step, oy + ry * step, Math.ceil(step), Math.ceil(step));
      }
    }
    ctx.drawImage(src, 0, 0, 64, 64, ox, oy, side, side);
    return;
  }

  const unit = Math.floor(Math.min((w - 36) / 16, (h - 32) / 32));
  const originX = Math.floor((w - 16 * unit) / 2);
  const originY = Math.floor((h - 32 * unit) / 2) - 2;

  // Voxel ground shadow pedestal
  const pedX = originX + 2 * unit;
  const pedY = originY + 31 * unit + Math.floor(unit * 0.45);
  const pedW = 12 * unit;
  ctx.fillStyle = "rgba(0, 0, 0, 0.28)";
  ctx.fillRect(pedX, pedY, pedW, Math.max(4, Math.floor(unit * 1.15)));
  ctx.fillStyle = "rgba(255, 255, 255, 0.09)";
  ctx.fillRect(pedX, pedY, pedW, 1);

  const blit = (
    sx: number,
    sy: number,
    sw: number,
    sh: number,
    dxUnits: number,
    dyUnits: number,
    dwUnits: number,
    dhUnits: number
  ) => {
    ctx.drawImage(
      src,
      sx,
      sy,
      sw,
      sh,
      originX + dxUnits * unit,
      originY + dyUnits * unit,
      dwUnits * unit,
      dhUnits * unit
    );
  };

  const armW = isSlim ? 3 : 4;
  const leftScreenArmDx = isSlim ? 1 : 0;
  const rightScreenArmDx = 12;

  if (mode === "front") {
    // Legs base + pants overlay
    blit(4, 20, 4, 12, 4, 20, 4, 12);
    blit(4, 36, 4, 12, 4, 20, 4, 12);
    blit(20, 52, 4, 12, 8, 20, 4, 12);
    blit(4, 52, 4, 12, 8, 20, 4, 12);

    // Torso base + jacket overlay
    blit(20, 20, 8, 12, 4, 8, 8, 12);
    blit(20, 36, 8, 12, 4, 8, 8, 12);

    // Arms (Right Arm on viewer's left, Left Arm on viewer's right) + sleeve overlays
    blit(44, 20, armW, 12, leftScreenArmDx, 8, armW, 12);
    blit(44, 36, armW, 12, leftScreenArmDx, 8, armW, 12);
    blit(36, 52, armW, 12, rightScreenArmDx, 8, armW, 12);
    blit(52, 52, armW, 12, rightScreenArmDx, 8, armW, 12);

    // Head base + hat overlay
    blit(8, 8, 8, 8, 4, 0, 8, 8);
    blit(40, 8, 8, 8, 4, 0, 8, 8);
  } else {
    // Legs back + pants back overlay
    blit(28, 52, 4, 12, 4, 20, 4, 12);
    blit(12, 52, 4, 12, 4, 20, 4, 12);
    blit(12, 20, 4, 12, 8, 20, 4, 12);
    blit(12, 36, 4, 12, 8, 20, 4, 12);

    // Torso back + jacket back overlay
    blit(32, 20, 8, 12, 4, 8, 8, 12);
    blit(32, 36, 8, 12, 4, 8, 8, 12);

    // Arms back + sleeve back overlay
    const leftArmBackU = isSlim ? 43 : 44;
    const leftSleeveBackU = isSlim ? 59 : 60;
    const rightArmBackU = isSlim ? 51 : 52;
    blit(leftArmBackU, 52, armW, 12, leftScreenArmDx, 8, armW, 12);
    blit(leftSleeveBackU, 52, armW, 12, leftScreenArmDx, 8, armW, 12);
    blit(rightArmBackU, 20, armW, 12, rightScreenArmDx, 8, armW, 12);
    blit(rightArmBackU, 36, armW, 12, rightScreenArmDx, 8, armW, 12);

    // Head back + hat back overlay
    blit(24, 8, 8, 8, 4, 0, 8, 8);
    blit(56, 8, 8, 8, 4, 0, 8, 8);
  }
}

function hashSeed(key: string): number {
  let seed = 7331;
  for (let i = 0; i < key.length; i += 1) {
    seed = (seed * 33 + key.charCodeAt(i)) >>> 0;
  }
  return seed;
}

function buildVoxelRidgePoints(
  width: number,
  height: number,
  cols: number,
  seed: number,
  baseY: number,
  amp: number,
  phase: number
): string {
  const step = width / cols;
  const pts: string[] = [`0,${height}`, `0,${baseY}`];
  for (let i = 0; i < cols; i += 1) {
    const wave =
      Math.sin((i + (seed % 11) + phase) * 0.55) * amp +
      Math.cos((i * 1.4 + (seed % 7) + phase * 1.7) * 0.42) * (amp * 0.55);
    const snapY = Math.round((baseY - wave) / 8) * 8;
    const clampedY = Math.max(24, Math.min(height - 20, snapY));
    const x1 = Math.round(i * step);
    const x2 = Math.round((i + 1) * step);
    pts.push(`${x1},${clampedY}`, `${x2},${clampedY}`);
  }
  pts.push(`${width},${height}`);
  return pts.join(" ");
}

export function renderClientStratumSvg(
  svg: SVGSVGElement,
  key: string,
  highlightActive: boolean
): void {
  const ns = "http://www.w3.org/2000/svg";
  while (svg.firstChild) {
    svg.removeChild(svg.firstChild);
  }

  const W = 480;
  const H = 220;
  const seed = hashSeed(key);

  const add = (
    tag: string,
    attrs: Record<string, string>,
    parent: SVGElement = svg
  ): SVGElement => {
    const el = document.createElementNS(ns, tag);
    for (const [k, v] of Object.entries(attrs)) {
      el.setAttribute(k, v);
    }
    parent.appendChild(el);
    return el;
  };

  // Architectural sky canvas background + restrained subtle orthographic datum grid
  add("rect", { x: "0", y: "0", width: String(W), height: String(H), fill: "var(--bg-canvas)" });
  add("rect", { x: "0", y: "0", width: String(W), height: "132", fill: "var(--accent)", opacity: "0.065" });

  for (let x = 32; x < W; x += 32) {
    add("line", {
      x1: String(x),
      y1: "0",
      x2: String(x),
      y2: String(H),
      stroke: "var(--border-subtle)",
      "stroke-width": "1",
      opacity: x % 96 === 0 ? "0.48" : "0.22",
    });
  }
  for (let y = 32; y < H; y += 32) {
    add("line", {
      x1: "0",
      y1: String(y),
      x2: String(W),
      y2: String(y),
      stroke: "var(--border-subtle)",
      "stroke-width": "1",
      opacity: "0.22",
    });
  }

  // Pixel celestial moon / sun square with restrained orbital ring
  const sunX = 290 + (seed % 110);
  const sunY = 34 + ((seed >>> 4) % 28);
  add("rect", {
    x: String(sunX - 18),
    y: String(sunY - 18),
    width: "36",
    height: "36",
    fill: "none",
    stroke: "var(--accent-fg)",
    "stroke-width": "1.2",
    opacity: "0.32",
  });
  add("rect", {
    x: String(sunX - 9),
    y: String(sunY - 9),
    width: "18",
    height: "18",
    fill: "var(--accent)",
    opacity: highlightActive ? "0.85" : "0.52",
  });

  // Distant voxel mountain peaks (stepped polygonal strata)
  add("polygon", {
    points: buildVoxelRidgePoints(W, H, 18, seed, 126, 36, 0.4),
    fill: "var(--border-subtle)",
    opacity: "0.46",
  });

  // Mid slate ridge
  add("polygon", {
    points: buildVoxelRidgePoints(W, H, 15, seed + 19, 148, 28, 1.8),
    fill: "var(--brand-left)",
    opacity: "0.84",
  });

  // Stepped obsidian foreground plateau + blocky trees / basalt pillars
  const fgCols = 20;
  const fgStep = W / fgCols;
  for (let i = 0; i < fgCols; i += 1) {
    const wave = Math.sin((i + (seed % 9)) * 0.48) * 14 + Math.cos(i * 0.85) * 6;
    const h = Math.round((56 + wave) / 8) * 8;
    const topY = H - h;
    const x = i * fgStep;

    add("rect", {
      x: String(x),
      y: String(topY),
      width: String(fgStep + 0.6),
      height: String(h),
      fill: "var(--brand-right)",
    });

    const isAccentRim = highlightActive && (i + (seed % 6)) % 5 === 0;
    add("rect", {
      x: String(x),
      y: String(topY),
      width: String(fgStep + 0.6),
      height: "4",
      fill: isAccentRim ? "var(--accent)" : "var(--text-subtle)",
      opacity: isAccentRim ? "0.92" : "0.52",
    });

    // Voxel crystal / spruce spire accents on select columns
    if ((i + (seed % 7)) % 6 === 2 && topY > 96) {
      add("rect", {
        x: String(x + 9),
        y: String(topY - 18),
        width: "6",
        height: "18",
        fill: "var(--brand-left)",
      });
      add("rect", {
        x: String(x + 4),
        y: String(topY - 26),
        width: "16",
        height: "10",
        fill: highlightActive ? "var(--accent-fg)" : "var(--border-control)",
        opacity: "0.78",
      });
    }

    // Ore vein pixel inlay
    if (highlightActive && (i + (seed % 5)) % 4 === 1 && topY + 26 < H - 10) {
      add("rect", {
        x: String(x + 6),
        y: String(topY + 18),
        width: "5",
        height: "5",
        fill: "var(--accent)",
        opacity: "0.88",
      });
      add("rect", {
        x: String(x + 12),
        y: String(topY + 24),
        width: "4",
        height: "4",
        fill: "var(--accent-fg)",
        opacity: "0.65",
      });
    }
  }

  // Orthographic registration crosshair + coordinate tick top-right
  add("path", {
    d: "M446 20v12M440 26h12",
    stroke: "var(--text-subtle)",
    "stroke-width": "1.2",
    opacity: "0.65",
  });
}

export function renderDeckCardThumbSvg(
  svg: SVGSVGElement,
  key: string,
  kind: "launch" | "world" | "server" | "nav"
): void {
  const ns = "http://www.w3.org/2000/svg";
  while (svg.firstChild) {
    svg.removeChild(svg.firstChild);
  }

  const W = 240;
  const H = 84;
  const seed = hashSeed(key);
  const tone =
    kind === "server"
      ? "var(--success)"
      : kind === "nav"
      ? "var(--text-subtle)"
      : "var(--accent)";

  const add = (tag: string, attrs: Record<string, string>) => {
    const el = document.createElementNS(ns, tag);
    for (const [k, v] of Object.entries(attrs)) {
      el.setAttribute(k, v);
    }
    svg.appendChild(el);
  };

  add("rect", { x: "0", y: "0", width: String(W), height: String(H), fill: "var(--bg-canvas)" });
  add("rect", { x: "0", y: "0", width: String(W), height: "48", fill: tone, opacity: "0.08" });

  for (let x = 24; x < W; x += 24) {
    add("line", {
      x1: String(x),
      y1: "0",
      x2: String(x),
      y2: String(H),
      stroke: "var(--border-subtle)",
      "stroke-width": "1",
      opacity: "0.24",
    });
  }

  if (kind === "server") {
    // Isometric voxel signal beacon + relay pulses
    add("polygon", {
      points: "0,84 0,60 72,60 96,48 164,48 192,62 240,62 240,84",
      fill: "var(--brand-right)",
      opacity: "0.9",
    });
    add("rect", { x: "115", y: "24", width: "10", height: "38", fill: "var(--brand-left)" });
    add("rect", { x: "112", y: "18", width: "16", height: "8", fill: tone, opacity: "0.9" });
    add("path", {
      d: "M102 18a22 22 0 0 1 36 0M94 12a32 32 0 0 1 52 0",
      fill: "none",
      stroke: tone,
      "stroke-width": "1.6",
      opacity: "0.65",
    });
  } else if (kind === "nav") {
    // Architectural wireframe cube + datum lines
    add("rect", { x: "20", y: "18", width: "36", height: "36", fill: "none", stroke: "var(--border-control)", "stroke-width": "1.2" });
    add("rect", { x: "32", y: "28", width: "36", height: "36", fill: "var(--brand-left)", opacity: "0.6", stroke: "var(--accent-fg)", "stroke-width": "1" });
    add("line", { x1: "92", y1: "30", x2: "212", y2: "30", stroke: "var(--border-subtle)", "stroke-width": "1.5" });
    add("line", { x1: "92", y1: "42", x2: "186", y2: "42", stroke: "var(--border-subtle)", "stroke-width": "1.5" });
    add("line", { x1: "92", y1: "54", x2: "152", y2: "54", stroke: tone, "stroke-width": "2" });
  } else {
    // Voxel horizon mountain & sun emblem per world / launch tile
    const sx = 166 + (seed % 40);
    add("rect", { x: String(sx), y: "14", width: "14", height: "14", fill: tone, opacity: "0.8" });
    add("polygon", {
      points: buildVoxelRidgePoints(W, H, 12, seed, 52, 16, 0.5),
      fill: "var(--brand-left)",
      opacity: "0.78",
    });
    add("polygon", {
      points: buildVoxelRidgePoints(W, H, 10, seed + 13, 64, 10, 1.9),
      fill: "var(--brand-right)",
    });
    add("rect", { x: "0", y: "80", width: String(W), height: "4", fill: tone, opacity: "0.55" });
  }
}
