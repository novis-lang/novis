- **Changing what a request's params are breaks the extension silently.** `regions()` in
  `editors/vscode/src/regions.ts` wraps its `sendRequest` in a `try`/`catch` returning `[]`, so params
  the server refuses read as "this file has no markup" and the HTML services stop forwarding, with
  neither `cargo test` nor `npm run compile` seeing it. Grep `editors/vscode/src` for the `METHOD`
  constant of any request whose dispatch arm in `crates/nvs-lsp/src/server.rs` you retype, and land
  both sides together. [until: gone editors/vscode/src/regions.ts:} catch {]
