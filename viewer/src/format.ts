// How the viewer writes a number of bytes and a delay. Plain TypeScript with no JSX, so a unit test imports it.

/** A number of bytes the way a person reads it: `512 B`, `1.50 KiB`, `12.3 MiB` or `256 MiB`. */
export function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value < 10 ? 2 : value < 100 ? 1 : 0)} ${units[unit]}`;
}

/** A delay in milliseconds the way a person reads it: `250 ms`, `30 s`, `5 min`, `2 h` or `3 d`. */
export function delay(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const steps: [number, string][] = [
    [86_400_000, "d"],
    [3_600_000, "h"],
    [60_000, "min"],
    [1000, "s"],
  ];
  const [unit, name] = steps.find(([unit]) => ms >= unit) ?? [1000, "s"];
  return `${Math.round(ms / unit)} ${name}`;
}
