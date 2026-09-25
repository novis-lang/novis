- **A hostile case that builds a big array with `Core\Arr::append` in a loop never finishes**: a million
  rounds hangs, and fifty thousand took two minutes where ten thousand takes five seconds. Each call
  copies the array it is given, so the loop is quadratic in the number of entries. Build a large
  array with `Core\Arr::fill(n, v)`, and keep an appending loop to about ten thousand rounds.
  [until: gone crates/nvs-stdlib/src/arr.rs:Core\Arr::append]
