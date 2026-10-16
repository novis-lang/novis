`nvs ext new` creates an extension project, and `nvs ext build` makes the `.nvsx` file from it.

An extension adds one class to Novis. You write its code in Rust, C or another language that
compiles to WebAssembly. `nvs ext new --lang rust <dir>` or `nvs ext new --lang c <dir>` writes a
project with one function, a test and a README. You build the project with your own compiler. Then
`nvs ext build` reads `nvsx.toml`, puts the wasm file and the Novis source files into one `.nvsx`
file, and checks that the file loads. It prints the path of the file and its `sha256`.

A program uses the class after `nvs.toml` lists the file in an `[[extension]]` entry with that
`sha256`.

**Good to know:** `nvs ext build` does not compile your code. Build for `wasm32-wasip2`, because a
WASI preview 1 module does not build. The same project always gives the same bytes.
