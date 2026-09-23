- **A `Core` member that `python tools/dossier.py --id` reports as owing a Rust test can already
  have one, under a name that never mentions it.** `Core\Cli::live` read `0 Rust` while
  `a_live_region_is_scoped_and_restores_the_terminal_on_a_panic` had pinned it all along, because a
  test is attributed by its `// covers:` marker and by nothing else
  (`rule:testing/proof-attribution`). Grep the member's own module for a `#[test]` about its subject
  before writing one: adding the marker to the test that exists is usually the whole edit, and a
  second test earns its place only by pinning something the first does not.
  [until: reviewed 2026-09-21]
