- **`cargo build --release` by hand cannot replace `target/release/nvs.exe` while an editor's `nvs
  lsp` holds it**, and on Windows it fails with `failed to remove file ... Zugriff verweigert (os
  error 5)`. A `nvs.path` pointing at that binary is a lock on it for the life of the window, so a
  release build made to test a server change silently leaves the old binary in place and the editor
  keeps answering from it. `tools/dossier.py` and `tools/loop.py` retry that failure once with the
  old binary renamed aside (`tools/relink.py`), so a release build through either owes nothing — by
  hand, stop the server first with `nvs.lsp.enable` set to `false`, or close the window.
  [until: gone editors/vscode/src/extension.ts:const SUBCOMMAND]
