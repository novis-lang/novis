The `[grants]` table says which capabilities the code in each namespace may use. A capability is a
right such as `fs.read` or `net.connect`. The table is in the `package.toml` file beside the entry
file of your application.

Each key is a namespace, and each value is a list of capabilities. `'Shop\Catalog'` matches only that
namespace. `'Shop\Catalog\*'` also matches every namespace below it. An empty list gives no
capability.

The check happens when the program compiles. A call compiles only if the calling namespace has the
capability it needs. When several keys match, the longest key wins. A namespace that no key matches
has every capability of the application.

`Core\Env::get` needs `env`. `spawn script`, `Core\Queue::push` and the `upgrade` methods need
`script.spawn`.

A package lists what it needs in its own `package.toml`. A capability in `required` must be on its
line. A capability in `optional` may be missing. Then the call throws a `RuntimeError`, and
`Core\Cap::has` returns `false`.

**Good to know:** the table only takes capabilities away. A call can do only what both `package.toml`
and `nvs.toml` allow.
