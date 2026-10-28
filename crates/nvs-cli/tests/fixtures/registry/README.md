# The test registry

No registry is committed here. A test builds one in a scratch folder with
`crates/nvs-cli/src/package/fixture.rs`, which packs each package directory through
`pack::file_set` and `pack::write`, so every digest in it is the one the code packs today. The
schema of its files is `rule:packaging/a-package-is-its-digest`'s.

```text
<scratch>/
  registry/registry.json             the root file the registry's URL serves
  files/index/<vendor>/<name>.json   one index file per package
  files/archive/<digest>.tar         one archive per version
  files/log/checkpoint, files/log/tile/...   the log, one publish record per version
  packages/<vendor>-<name>-<version>/   the package directories the archives are packed from
```

Each fixture makes its own log key, and an application names it with `Fixture::key`. Each index
line's `log` is its publish record's number. The root file lists `files/` as its one file host. A test that needs more hosts, such as one that
does not exist, rewrites the list with `Fixture::hosts`.
