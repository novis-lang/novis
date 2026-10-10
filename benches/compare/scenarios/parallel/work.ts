// One part of the work: the same loop every peer runs.
export function part(seed: number, turns: number): number {
  let x = seed;
  for (let i = 0; i < turns; i++) {
    x = (x * 31 + i) % 1000003;
  }
  return x;
}
