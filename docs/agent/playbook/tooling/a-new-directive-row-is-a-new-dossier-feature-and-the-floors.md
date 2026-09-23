- **A new directive row is a new dossier feature, and the floor's `--emit-goals --dry-run` check turns
  red until its proofs exist.** Splitting `listen`, `socket_mode` and `workers` out of `[server]` gave
  `tools/dossier.py` three `directive:server.*` features no goal on disk claims, so the check read
  "would append 1 goal(s)" with no feature named. Run `python tools/dossier.py --verify --only
  directive:<key>` for every row a change adds, and write its example, attack and `// covers:` marker
  in the same slice. [until: gone tools/dossier.py:claimed_features]
