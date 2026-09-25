- **An allocating `Core` member can end the program rather than throw, because a request's memory
  ceiling is a `FATAL` and no `try` catches it.** `Core\Arr::fill` throws a catchable error for a
  count nothing could ever hold, and `Core\Arr::fill(4000000000, 'x')` instead reaches the ceiling,
  which ends the process and takes every later step of the attack with it. Run a new attack once
  with `target/debug/nvs.exe run` before fixing its order: the step that ends the program goes last,
  and the file declares `// hostile: ends-early`. [until: gone tests/hostile/README.md:ends-early]
