---
id: packages
title: "Packages: grants and package files"
summary: the `[grants]` table in an application's `package.toml`, which says what each namespace may do, and the rules every file of a package follows
keywords: package.toml, grants, [grants], namespace, package, vendor, dependency, capability, capabilities, required, optional, Core\Cap::has, E0655, E0656, E0657, E0658, E0659, E0660, E0661, permission, supply chain, third-party code
---

<!-- src: `rule:security/grants-are-keyed-on-a-namespace`, `rule:security/authority-is-the-enclosing-namespace`, `rule:security/longest-prefix-wins`, `rule:security/package-authority-is-granted-one-line-at-a-time`, `rule:security/optional-capability-degrades`, `rule:packaging/a-package-file-stays-under-its-declared-prefix`, `rule:packaging/a-package-cannot-reach-its-host` -->

# Grants: what each namespace may do

The `[grants]` table says which capabilities the code in each namespace may use. A capability is a
right such as `fs.read` or `net.connect`. The table is in the `package.toml` file of your
application:

```toml
[grants]
'Acme\Http\*' = ["net.connect"]
'Acme\Cache'  = []
```

Novis finds this file in the directory of the entry file, or in the nearest directory above it. The
entry file is the file you pass to `nvs run` or `nvs serve`. The current directory does not matter.

Each key is a namespace. Each value is a list of capability names. `'Acme\Cache'` matches only the
namespace `Acme\Cache`. `'Acme\Http\*'` matches `Acme\Http` and every namespace below it, such as
`Acme\Http\Client`. An empty list gives the namespace no capability.

The check happens when the program compiles. A call to a `Core` method that needs a capability
compiles only if the namespace of the code that makes the call has that capability. The namespace is
the one in the file's `namespace` line. An anonymous function has the namespace it is written in. A
`use` line does not change anything. A missing capability is error `E0658`. The message shows the
line to change.

```toml file=package.toml
[grants]
'Acme\Cache' = []
```
```nvs file=lib/Store.nvs
<?nvs
namespace Acme\Cache;

final class Store
{
    public static function load(): string
    {
        return Core\IO::readText("note.txt");
    }
}
```
```nvs error
<?nvs
autoload 'Acme\Cache' from './lib';

echo Acme\Cache\Store::load(), "\n";
```
```output
the namespace `Acme\Cache` does not have the capability `fs.read`
```

When several keys match a namespace, the longest key wins. Length is counted in namespace parts. A
key without `\*` wins over a key with `\*` of the same length. Only the winning line applies. A
longer key never adds to a shorter one.

A namespace that no key matches has every capability of the application. So a program without a
`[grants]` table compiles exactly as before. A namespace with a line has only what the line lists.

The table can only take capabilities away. `nvs.toml` still decides what the program may do at
all, and each path or host a call uses is still checked when the call runs. A call can do only what
both files allow.

These lines are errors:

| Line | Error |
|---|---|
| a key that is not a namespace, a value that is not a list, an unknown capability | `E0655` |
| `'\Acme\Http'` next to `'Acme\Http'`: a `\` at the start changes nothing, so both keys are the same | `E0656` |
| a key that names `Core` or a namespace below it | `E0657` |

## A package's own `package.toml`

A package is a directory with its own `package.toml`. Its name is a vendor and a name, such as
`Acme\Disk`. The file lists the capabilities the package needs:

```toml
[package]
name = 'Acme\Disk'

[capabilities]
required = ["fs.read"]
optional = ["fs.write"]
```

This list grants nothing. It only tells you what to write in `[grants]`. A file of your program
belongs to the package when the package's `package.toml` is in the file's directory or in a directory
between the file and the entry file. A `package.toml` in the entry file's directory, or above it, is
your application's own file. A bad package file is error `E0659`.

A capability in `required` that the package's grant line does not list is error `E0658`, as above.
A capability in `optional` that the line does not list compiles. A call that needs it throws a
`RuntimeError` when it runs. Before the call, the package can check the capability with
`Core\Cap::has`:

```nvs skip
if (Core\Cap::has("fs.write")) {
    Core\IO::write("cache.txt", $text);
}
```

`Core\Cap::has` returns `true` only when the grant line of the calling namespace and `nvs.toml`
both allow the capability.

## The rules for a package's files

Every file of a package declares the package's namespace or a namespace below it. A file of
`Acme\Disk` may declare `namespace Acme\Disk;` or `namespace Acme\Disk\Index;`. Any other
namespace is error `E0660`. So a package cannot use the grant line of your application or of another
package.

A `require` or `autoload` path in a package must stay inside the package's directory. A path that
leaves it with `..`, with an absolute path or through a symbolic link is error `E0661`.
