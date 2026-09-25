// `bun nv exe-icons`: cuts the two icons `crates/nvs-cli/build.rs` links into `nvs.exe`.
//
//     bun nv exe-icons
//
// `website/media/novis-logo.png` becomes `crates/nvs-cli/assets/nvs.ico`, the release binary's icon, and
// `website/media/novis-logo-file-icon-light-theme-nvst.svg` becomes `nvs-debug.ico`, the debug binary's.
// Both are committed, so a build needs neither this command nor ImageMagick; run it after either drawing
// changes and commit what it writes. `rule:packaging/the-windows-binary-says-what-it-is` owns which binary
// carries which.
//
// It needs ImageMagick's `magick` on `PATH`, the tool `website/README.md` § *Logo and favicon* already
// cuts the favicon with, and for the same two reasons: each size is trimmed of the export's transparent
// margin and sharpened for the size it is, which `-define icon:auto-resize` cannot do.

import { mkdirSync } from "node:fs";
import { join, parse } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary = "cut nvs.exe's release and debug icons from the website's drawings with ImageMagick: nv exe-icons";

const MEDIA = join(ROOT, "website", "media");
const ASSETS = join(ROOT, "crates", "nvs-cli", "assets");
const SCRATCH = join(ROOT, ".agent-tmp", "exe-icons");

const ICONS: Record<string, string> = {
  "nvs.ico": join(MEDIA, "novis-logo.png"),
  "nvs-debug.ico": join(MEDIA, "novis-logo-file-icon-light-theme-nvst.svg"),
};

// The sizes Explorer, the taskbar and Alt-Tab pick from at 100% to 250% scaling, each with the unsharp
// amount that keeps the mark's counters open at that size.
const SIZES: [number, string | null][] = [
  [16, "0x0.6+0.8+0.02"],
  [20, "0x0.6+0.7+0.02"],
  [24, "0x0.6+0.7+0.02"],
  [32, "0x0.6+0.6+0.02"],
  [40, "0x0.6+0.5+0.02"],
  [48, "0x0.6+0.5+0.02"],
  [64, "0x0.6+0.4+0.02"],
  [256, null],
];

async function magick(...args: string[]): Promise<void> {
  const r = await runProc(["magick", ...args]);
  if (r.code !== 0) throw new Error(`magick ${args.join(" ")} failed (exit ${r.code}): ${r.stderr.trim()}`);
}

async function cut(source: string, target: string): Promise<void> {
  const stem = join(SCRATCH, parse(target).name);
  const mark = `${stem}-mark.png`;
  // A drawing is rasterised well above the largest size it is cut to; the density is ignored for the
  // PNG, which is already pixels.
  await magick("-background", "none", "-density", "384", source, "-trim", "+repage", mark);
  const frames: string[] = [];
  for (const [size, unsharp] of SIZES) {
    const frame = `${stem}-${size}.png`;
    const sharpen = unsharp ? ["-unsharp", unsharp] : [];
    await magick(mark, "-filter", "Lanczos", "-resize", `${size}x${size}`, ...sharpen, "-background", "none", "-gravity", "center", "-extent", `${size}x${size}`, frame);
    frames.push(frame);
  }
  await magick(...frames, target);
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  if (Bun.which("magick") === null) {
    console.error("nv exe-icons: ImageMagick's `magick` is not on PATH");
    return 1;
  }
  mkdirSync(SCRATCH, { recursive: true });
  mkdirSync(ASSETS, { recursive: true });
  for (const [name, source] of Object.entries(ICONS)) {
    const target = join(ASSETS, name);
    try {
      await cut(source, target);
    } catch (e) {
      console.error(`nv exe-icons: ${(e as Error).message}`);
      return 1;
    }
    console.log(`wrote ${rel(target)}`);
  }
  return 0;
}
