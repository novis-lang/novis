- **After touching either spec file, `bun nv migration`; after moving or renaming a doc, `bun nv
  links`.** The link check reports broken *and* mis-cased relative links, and
  `tools/nv/cmd/migration.ts` checks `docs/spec/02-php-migration.md` against the PHP inventory.
  [until: gone tools/nv/cmd/migration.ts]
