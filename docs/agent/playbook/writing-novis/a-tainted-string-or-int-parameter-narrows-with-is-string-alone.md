- **A `tainted string|int` parameter narrows with `is string` alone.** The `else` of
  `$x is int` still has `$x` as `int|tainted string` (E0401 at `Core\Taint::assertTrusted`), and
  `$x is tainted string` fails E0813. Write `if ($x is string)`, which gives `tainted string`, as
  `extensions/spreadsheet/nvs/Reader.nvs`'s `sheetName` does. A `Novis\` class's source is embedded
  at build time, so each try costs a `cargo build`. `until: gone extensions/spreadsheet/nvs/Reader.nvs:private function sheetName(tainted string|int $sheet)`
