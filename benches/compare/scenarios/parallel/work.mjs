// One part of the work: the same loop every peer runs.
export function part(seed, turns) {
  let x = seed;
  for (let i = 0; i < turns; i++) {
    x = (x * 31 + i) % 1000003;
  }
  return x;
}
