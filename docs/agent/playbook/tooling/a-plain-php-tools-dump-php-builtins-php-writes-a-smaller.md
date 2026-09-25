- **A plain `php tools/dump-php-builtins.php` writes a *smaller* inventory than the committed one, and
  growing it drops `bun nv migration` under its `--min 100` floor.** The Windows build ships
  `fileinfo` and `zip` as DLLs its `php.ini` does not enable, and every name a regeneration adds is
  `open` until `docs/spec/02-php-migration.md` rows it. Pass `-d extension=` per DLL, diff the
  `# extensions:` header, and row the new names in the same slice.
  [until: gone tools/dump-php-builtins.php:fileinfo]
