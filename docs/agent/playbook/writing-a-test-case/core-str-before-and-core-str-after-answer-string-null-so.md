- **`Core\Str::before` and `Core\Str::after` answer `string|null`, so neither can be handed
  straight to a `Core` parameter typed `string`.** An example that cuts a header at its first `:`
  compiles until the piece is passed on, and the `E0401` then names the *receiving* call rather
  than the splitter that produced the union. Use `Core\Str::slice` or `Core\Str::split` wherever
  the piece goes into another member, and keep `before`/`after` for what is echoed.
  [until: gone crates/nvs-stdlib/src/str.rs:Core\Str::before]
