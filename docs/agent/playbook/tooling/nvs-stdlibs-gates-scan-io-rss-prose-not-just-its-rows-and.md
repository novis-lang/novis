- **nvs-stdlib's gates scan `io.rs`'s *prose*, not just its rows, and two of them read a doc comment
  as code.** `no_member_dispatches_on_a_uri_scheme` fails on the string `php://stdin` inside a
  reference card's `short`, because the scan is for the scheme spelling anywhere in the module, and
  a `-p nvs-stdlib --test capability` failure naming a doc-comment line reads like a code bug;
  `no_registry_card_cites_an_adr` is the same shape one file over. Name what PHP's spelling *did*
  ("the standard-input wrapper") rather than writing it. [until: reviewed 2026-09-06]
