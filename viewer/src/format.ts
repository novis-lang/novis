// How the viewer writes a number of bytes. Plain TypeScript with no JSX, so a unit test imports it.

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
