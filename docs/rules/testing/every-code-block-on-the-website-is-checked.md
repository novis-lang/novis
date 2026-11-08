Every fenced block of a hand-written website page is run by the check its language has, or its fence
says why it is not: ```` ```sh skip="installs a system service" ````. The marker is the fence's meta,
where ```` ```nvs skip ```` already is, and a `skip` with no reason fails. A block with no language, or
in a language no check knows, fails too. `bun nv site --check code` walks the pages no renderer
writes, and names each block that fails as `page:line`. A block that fails its check is corrected on
the page, never marked `skip`. `tools/nv/cmd/site.ts` § *A code block* is the home of the markers and
of `CODE_CHECKS`, the table of what checks each language.

What checks each language:

- **`toml`** is written to a scratch file and read by `nvs config check`. A block that is part of a
  file says which table with `fragment="<table>"`. A file the block names that the checking machine
  does not have gets a one-line stub in the block's scratch folder, an absolute path rewritten under
  it, and the block is read again, because `config check` stops at the first missing file and would
  hide every other error.
- **`json`** is parsed, inside `{ }` when its fence says `fragment`. **`yaml`** is parsed by
  `Bun.YAML.parse`.
- **`sh`, `bat` and `powershell`**: each `nvs` line is parsed by `nvs` itself under
  `NOVIS_PARSE_ONLY`, which exits 0 once the arguments parse and before anything runs. Each `bun nv`
  line must name one of `bun nv`'s commands. A block with any other program carries `skip`, and a
  `skip` covers the whole block.
- **`txt`** is output: `run="nvs ..."` names the command that prints it and `cwd` where it runs, and
  its lines appear in that output in order, so a page may show an excerpt. A file tree or a table of
  text carries `skip`.
- **`nginx`, `caddyfile` and `dockerfile`** are checked by `nginx -t`, `caddy validate` and
  `docker build --check`, each in its official image pinned by digest (`bun nv proxy-validate`). The
  text reaches the container on standard input. A Caddyfile that needs a plugin names it with
  `plugins="<module>"`, and Caddy is built with that pinned plugin once per machine. Where Docker runs
  no Linux containers, these checks print why once and pass, so CI's Linux leg is the one that judges
  them.
- **`rust`** names the file it is copied from with `copy="<path>"`, and its lines are a run of that
  file's lines. **`md`** is text the reader types into a demo, and carries `skip`.

`bun nv site --check apps` holds each example app's `deploy/` folder to the same checks:
`nvs config check` reads `nvs.toml` with `deploy/production.toml`, and its `Caddyfile` and
`nginx.conf` go through `bun nv proxy-validate`. CI's website stale guard and `bun nv ci --check` run
both parts.

- **Memory:** nothing in the language or the binary. A check run pulls the pinned nginx, Caddy and
  Docker images once, and builds the plugin's Caddy image once per machine, about a minute.

**Declined:**
- **A `toml` check that tolerates a missing file.** `config check` stops at the first one, so the
  rest of the block would go unchecked.
- **A `config check` flag that skips file reads.** It would weaken the command for everybody, and
  only this check needs it.
- **Turning a `txt` block into a `<Terminal>`.** A terminal's fixture is a copy, so the app or snippet
  the output comes from would have two homes.
- **Splitting a block that mixes `cargo` and `nvs` lines** so the `nvs` lines are parsed. It costs the
  reader a step for one parse.
- **Removing a Caddyfile's `rate_limit` block because it needs a plugin.** The page would lose the
  login limit it shows.
- **A CI step of its own for the proxy checks, beside veraPDF's.** The `code` and `apps` parts call
  `bun nv proxy-validate` themselves inside the website stale guard step, so a second step would run
  the same checks twice. That step's job has a 90-minute limit, which the plugin build fits.
