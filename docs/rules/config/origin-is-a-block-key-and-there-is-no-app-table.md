`origin` — what `Core\Router::urlAbsolute` prepends when the mount states none — and `mode` are keys
of the `[[app]]` block itself, read off every block matching the entry file and layered like any other
directive, the most specific block that states one winning.

There is **no global `[app]` table**, and there cannot be one: TOML forbids one file spelling `app` as
both a table and an array of tables, so a file holding `[app]` and `[[app]]` fails to parse outright. A
global origin would therefore be unspellable in exactly the multi-application deployments the fallback
exists for. A host-wide default is a block with the widest `root`, which
`rule:config/every-matching-app-block-applies-least-specific-first` already layers under the more
specific ones. `origin` keeps its class — `System`, and `Reload` on the restart axis — and
`rule:routing/an-absolute-link-takes-a-configured-origin` reads it from here and from nowhere else.
