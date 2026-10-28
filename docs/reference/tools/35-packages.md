---
id: packages
title: "Packages: grants, package.toml, adding and fetching packages, and API versions"
summary: the `[grants]` table in an application's `package.toml`, which says what each namespace may do, the rules every file of a package follows, every key of `package.toml`, `nvs pkg add`, `nvs pkg fetch` and `nvs pkg vendor`, which add a package and write it under `vendor/` with `package.lock`, and `nvs pkg api` and `nvs pkg diff`, which find every breaking change between two versions
keywords: package.toml, grants, nvs pkg add, nvs pkg fetch, nvs pkg vendor, package.lock, --locked, vendor, vendor/packages.nvs, registry, registries, dependencies, git dependency, digest, BLAKE3, released, retracted, withheld, unlisted, install, E0317, nvs pkg api, nvs pkg diff, public API, breaking change, version, semantic versioning, major, minor, patch, [grants], namespace, package, vendor, dependency, capability, capabilities, required, optional, Core\Cap::has, E0655, E0656, E0657, E0658, E0659, E0660, E0661, permission, supply chain, third-party code
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
        return Core\IO::readText('note.txt');
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
if (Core\Cap::has('fs.write')) {
    Core\IO::write('cache.txt', $text);
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

<!-- src: `rule:packaging/package-toml-is-a-tool-input`, `rule:packaging/a-package-name-is-vendor-slash-name`, `rule:packaging/a-vendor-resolves-from-one-registry`, `rule:packaging/a-git-dependency-is-root-only` -->

# package.toml

`package.toml` describes your application or a package. The `nvs pkg` commands read all of it.
When your program compiles, Novis reads only `[grants]` of your application's file, and only
`[package] name` and `[capabilities]` of a package's file. Your program never reads the file when it
runs.

A file with a `[package] name` is a package's file. A file without one is an application's file.
This is an application's file:

```toml
[dependencies]
'Acme\Slug' = "1.2.0"
'Acme\Fork' = { git = "https://git.example.com/acme/fork.git", rev = "4f2a9c0e1b7d3a5f6e8c9b0a1d2e3f4a5b6c7d8e" }

[registries]
Acme = { url = "https://packages.example.com" }

[grants]
'Acme\Slug\*' = []  # required: fs.read
'Acme\Fork\*' = []
```

| Table or key | What it says |
|---|---|
| `[package] name` | the package's name, which is also its namespace, such as `'Acme\Slug'` |
| `[package] version` | the package's version, `MAJOR.MINOR.PATCH`, such as `"1.2.0"` |
| `[package] files` | the files the package carries beside its `src` folder, such as `["data/words.txt"]` |
| `[dependencies]` | each package your code needs, with the lowest version it works with |
| `[registries]` | the registry each vendor's packages come from |
| `[capabilities]` | the capabilities a package needs, as `required` and `optional` |
| `[grants]` | the capabilities each namespace may use, in an application's file only |
| `[[download]]` | a large file a package downloads from another host, with its size and SHA-256 hash |

`version`, `files`, `[capabilities]` and `[[download]]` are allowed only in a package's file. Any
other key is error `E0659`, and the `nvs pkg` commands stop.

A package's name has two parts, a vendor and a name: `Acme\Slug`. On the command line and in a URL,
you write it in lower case with `/`: `acme/slug`. No package may be named `Core` or take the name of a
built-in component, such as `Novis\Image`.

A dependency is written with its namespace, the same way as in `[grants]`. Its value is the lowest
version your code works with. `nvs pkg fetch` may choose a higher version, as
[nvs pkg fetch and nvs pkg vendor](#nvs-pkg-fetch-and-nvs-pkg-vendor) says.

A dependency can also come from a git repository. Write its URL and the full commit hash as `rev`. A
branch or a tag is not allowed, because it can point to other code later. Only your application's
file may have a git dependency. When a package's file has one, `nvs pkg fetch` stops with an error.

Each vendor's packages come from one registry. `[registries]` names the registry for a vendor. Every
other vendor's packages come from `https://registry.novis-lang.org`. A registry URL starts with
`https:`, or with `file:` for a folder on your disk. `nvs pkg fetch` never looks for a vendor's
package in a second registry. So nobody can publish a package with your vendor's name in another
registry and have it used.

<!-- src: `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`, `rule:packaging/a-known-bad-version-is-retracted-not-deleted` -->

# nvs pkg add

    nvs pkg add <package>
    nvs pkg add <package>@<version>

`nvs pkg add acme/slug` adds the newest released version of `acme/slug` to `[dependencies]`. Then it
does what `nvs pkg fetch` does. With a version, such as `acme/slug@1.2.0`, it adds exactly that
version.

Before the command changes any file, it prints each package it adds, with the capabilities that
package requests. The packages that `acme/slug` needs are included:

```text
`Acme\Slug` 1.2.0 needs 1 other package(s). Each one requests:
  `Acme\Slug` 1.2.0: required fs.read
  `Acme\Text` 1.0.0: no capabilities
There is no package.lock yet, so every package is new:
  `Acme\Slug` 1.2.0 is new. It requests required fs.read.
  `Acme\Text` 1.0.0 is new. It requests no capabilities.
added to [grants]: 'Acme\Slug\*' = []  # required: fs.read
added to [grants]: 'Acme\Text\*' = []
2 package(s), 2 written to vendor/
```

Each new grant line is empty, so the package may use no capability yet. The comment says what the
package requests. You add a capability to the line when you agree with it.

A new version of a package is released seven days after it is published, or earlier when an owner of
the package confirms it. `nvs pkg add` without a version chooses only a released version. A version
that waits for its release, or that its owners unlisted, is added only when you write its number.

A retracted version has a known problem. The command stops with an error that gives the reason and
the version with the fix. A version that the registry withheld, such as malware, also stops the
command. In both cases, no file changes.

A dependency that is already in `[dependencies]` keeps its version when that version is higher. With
`@<version>`, the command writes that version. The comment on the line stays.

When a package also downloads files from other hosts, the command lists each host and the total
size. Those hosts see the address of the computer that runs `nvs pkg fetch`.

<!-- src: `rule:packaging/a-fetched-package-is-an-autoload-line`, `rule:packaging/resolution-takes-the-highest-minimum`, `rule:packaging/the-lockfile-holds-every-digest`, `rule:packaging/nothing-runs-before-the-program`, `rule:packaging/a-package-carries-only-its-source-and-what-it-lists` -->

# nvs pkg fetch and nvs pkg vendor

    nvs pkg fetch [--locked]
    nvs pkg vendor [--locked]

`nvs pkg fetch` downloads every package in `[dependencies]`, and every package those packages need.
Run it in the folder of your application, next to `package.toml`. It writes these files:

| File | What it contains |
|---|---|
| `vendor/<vendor>/<name>/` | each package's files: `package.toml`, the `src` folder and the files it lists |
| `vendor/packages.nvs` | one `autoload` line for each package |
| `vendor/package-files.toml` | the files of each package, which its own code may read with no grant |
| `package.lock` | the version, the source and the digest of every package |
| `package.toml` | one new grant line in `[grants]` for each new package |

Your program loads every package with one line:

```nvs skip
require './vendor/packages.nvs';
```

`vendor/packages.nvs` looks like this:

```nvs skip
<?nvs
// Generated by `nvs pkg` from package.lock. Do not edit.
autoload 'Acme\Slug' from './acme/slug/src';
autoload 'Acme\Text' from './acme/text/src';
```

When two packages need different versions of a third package, the higher version is used. A version
is only chosen when a package asks for it, so a new release does not change your program until you
run `nvs pkg add` or `nvs pkg update`.

The digest of a package is the BLAKE3 hash of its archive. `package.lock` has the digest of every
package. Commit `package.lock` with your code. The next `nvs pkg fetch` checks each archive against its
digest and stops with an error when one does not match. `--locked` also stops when `package.lock` is
missing a package or has to change, so CI uses exactly the versions you tested.

No code of a package runs during `nvs pkg fetch`. A package's code runs only when your program calls
it. A file that `autoload` loads may only declare a class, an interface, an enum or a type. Any other
statement at the top of the file is error `E0317`.

A new grant line gives the package no capability. A line that is already in `[grants]` is never
changed, so a capability you added stays. A package's own code may read the files that
`vendor/package-files.toml` lists for it with no grant, when the path is a string in the code. It may
not write them.

`nvs pkg fetch` keeps a package folder whose files still match `package.lock`. `nvs pkg vendor` does the
same work, but it downloads every package again and replaces its folder. Use it when a file in
`vendor/` was changed. Both commands remove only the folders of packages they wrote before, so code
you copied into `vendor/` yourself stays.

# nvs pkg api and nvs pkg diff

    nvs pkg api <dir>
    nvs pkg diff <old> <new> [--from <version>] [--to <version>] [--json]

The public API of a package is every public class, interface, enum and type in it, with their
methods, properties and constants. A version number of a package is `MAJOR.MINOR.PATCH`, such as
`1.4.0`. From `0.1.0` on, only a major release may break the public API.

`nvs pkg api` prints the public API of the package in `<dir>` as JSON. It reads every `.nvs` file
under `<dir>/src` and checks the program. It does not run any of the code. Each type is written with
its full name, so an alias, a `use` line or a change in formatting is not a change.

`nvs pkg diff` compares two versions. `<old>` and `<new>` are each a package directory or a JSON file
that `nvs pkg api` printed. Each change prints on one line, as `breaking` or `additive`:

```text
breaking: Api\Client::send() — method removed
1 change(s): 1 breaking, 0 additive
1.2.0 to 1.3.0 is a minor release, so no breaking change is allowed.
```

A breaking change is a change that can stop code that uses the package from compiling:

- a public class, method, property, constant or enum case is removed or renamed;
- a method is added to an interface, or an abstract method is added;
- a case is added to an enum, because a `match` over every case no longer covers it;
- a parameter accepts fewer types, or a return type has more types;
- a required parameter is added, or a parameter is renamed.

A change that the comparison cannot decide is a breaking change. There is no way to allow one
breaking change and not the others.

The versions decide which changes are allowed. Each version is the `version` key in the
`package.toml` of the directory. `--from` and `--to` replace the two versions.

| From and to | Breaking changes |
|---|---|
| a `0.0.x` version on either side, such as `0.0.4` to `0.1.0` | allowed: `0.0.x` is experimental |
| a higher major, such as `1.2.0` to `2.0.0` | allowed |
| a higher minor or patch, such as `1.2.0` to `1.3.0` | not allowed |
| a new version that is not higher than the old one | not allowed |
| a side without a version | not allowed |

`nvs pkg diff` exits with `1` when a breaking change is not allowed, so you can run it in CI before
you publish. `--json` prints the changes as JSON. The JSON also has `from`, `to`, `step` and
`allowed`.

The check is about the shape of the API only. The behaviour of a method may change in any release.

<!-- src: `rule:packaging/only-a-major-may-break-the-api` -->
