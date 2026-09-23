- **A capability grant written as a list reads back through `Core\Config::get` exactly as an
  ungranted one does — as nothing at all.** `nvs_config::request`'s `as_text` turns a TOML scalar
  into text and answers `None` for an array, so an `[[app]]` fixture granting
  `fs.read = ["some/dir"]` makes a dossier example print `(nothing)` for the one grant the block
  exists to show, while a `read = true` beside it prints fine. Write a page's fixture grants as
  booleans and send the reader to `nvs config dump`, which is `nvs_config::audit` and does render a
  list. [until: gone crates/nvs-config/src/request.rs:as_text]
