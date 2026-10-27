The `[grants]` table says which capabilities the code in each namespace may use. A capability is a
right such as `fs.read` or `net.connect`. The table is in the `package.toml` file beside the entry
file of your application.

Each key is a namespace, and each value is a list of capabilities. `'Shop\Catalog'` matches only that
namespace. `'Shop\Catalog\*'` also matches every namespace below it. An empty list gives no
capability.

The check happens when the program compiles. A call that needs a capability compiles only if the
namespace of the calling code has it. When several keys match, the longest key wins. A namespace
that no key matches has every capability of the application.

A package lists what it needs in its own `package.toml`. A capability in `required` must be on the
package's line. A capability in `optional` may be missing. Then the call throws a `RuntimeError`,
and `Core\Cap::has` returns `false` before it.

**Good to know:** the table can only take capabilities away. `nvs.toml` still decides what the
program may do at all. A call can do only what both files allow.
