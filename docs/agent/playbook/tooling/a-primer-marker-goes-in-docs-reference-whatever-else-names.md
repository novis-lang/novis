- **A `<!-- primer -->` marker goes in `docs/reference/`, whatever else names `docs/spec/`.**
  `tools/reference.py`'s `SOURCES` is `docs/reference`, and the one spec file it reads is
  `docs/spec/02-php-migration.md` for Part D's crosswalk table — so a marker written into
  `docs/spec/` is read by nothing and lifts nothing into `nvs agent primer`. Put it on the line
  above a section heading under `docs/reference/lang/` or `docs/reference/tools/`, and expect the
  same slice to owe `python tools/reference.py --no-examples` and `docs/novis.md` in its commit,
  because every chapter edit makes the generated file stale.
  [until: gone docs/reference/README.md]
