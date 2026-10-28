---
id: packages
title: "Packages: grants, package.toml, adding, fetching, updating, packing and publishing packages, and API versions"
summary: the `[grants]` table in an application's `package.toml`, which says what each namespace may do, the rules every file of a package follows, every key of `package.toml`, `nvs pkg add`, `nvs pkg fetch` and `nvs pkg vendor`, which add a package and write it under `vendor/` with `package.lock`, `nvs pkg update` and `nvs pkg outdated`, `--locked`, `nvs pkg pack`, `nvs pkg download` and `nvs pkg provide`, the registry's transparency log and its signed index files, `nvs pkg api` and `nvs pkg diff`, which find every breaking change between two versions, and `nvs pkg login`, `nvs pkg whoami`, `nvs pkg publish`, `nvs pkg retract` and `nvs pkg unlist`, which publish a package and change its versions on the registry
keywords: package.toml, grants, nvs pkg add, nvs pkg fetch, nvs pkg vendor, nvs pkg update, nvs pkg outdated, nvs pkg pack, nvs pkg download, nvs pkg provide, [[download]], SHA-256, download cache, archive, package.lock, --locked, vendor, vendor/packages.nvs, registry, registries, dependencies, git dependency, digest, BLAKE3, released, retracted, withheld, unlisted, install, E0317, transparency log, checkpoint, signed note, Ed25519, fork, nvs registry key, nvs registry log append, nvs registry log verify, nvs registry index sign, nvs pkg api, nvs pkg diff, public API, breaking change, version, semantic versioning, major, minor, patch, [grants], namespace, package, vendor, dependency, capability, capabilities, required, optional, Core\Cap::has, E0655, E0656, E0657, E0658, E0659, E0660, E0661, permission, supply chain, third-party code, nvs pkg login, nvs pkg whoami, nvs pkg publish, nvs pkg retract, nvs pkg unlist, publish token, nvsreg_, pending, passkey, publish
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
Acme = { url = "https://packages.example.com", key = "packages.example.com+6c1a1f0e+AW0nQ8Kq3s9xUjvL4kz5Ff2hN1pYc7RbT0eGdMiSo8Ax" }

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

The `key` is the public key of the registry's log. You get it from the people who run the registry.
`nvs pkg` uses a package only if the log contains it. The log is a list of every change to the
registry, and nobody can rewrite it. If the log does not continue the one `nvs pkg` saw last time,
`nvs pkg` stops with an error and fetches nothing.

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

<!-- src: `rule:packaging/resolution-takes-the-highest-minimum`, `rule:packaging/a-known-bad-version-is-retracted-not-deleted` -->

# nvs pkg update and nvs pkg outdated

    nvs pkg update [<package> ...]
    nvs pkg outdated

`nvs pkg outdated` lists each package that has a newer released version. It changes no file:

```text
`Acme\Slug` 1.2.0 -> 1.3.0
`Acme\Text` 1.1.0 -> 2.0.0 (a new major version)
```

`nvs pkg update acme/slug` raises the minimum of `acme/slug` in `[dependencies]` to its newest released
version. Without a package, the command raises every dependency. A minimum is only raised, never
lowered. The comment on the line stays. Then the command does what `nvs pkg fetch` does.

Before the command changes any file, it prints what changed since `package.lock`:

```text
`Acme\Slug`: the minimum is raised from 1.2.0 to 1.3.0.
Changes since package.lock:
  `Acme\Slug` 1.2.0 -> 1.3.0
    now requests net.connect (optional)
    owners were acme and are now acme, newcomer
```

The report lists each new version, each capability a package now requests or no longer requests, a
change of owners and a change of repository. A new package and a package that is no longer needed are
listed too. The report changes no grant line, so a capability that a package now requests stays
refused until you add it.

When two packages need different major versions of a third package, the higher version is used. The
report then names the packages that asked for the older version. Your program is checked when it
compiles, so a change that breaks your code is a compile error.

A retracted version that `package.lock` already has stays in use, so you can update the other packages.
The command never chooses a retracted version that is not in `package.lock`. A git dependency is not
updated by these commands. Change its `rev` in `package.toml` instead.

<!-- src: `rule:packaging/the-lockfile-holds-every-digest`, `rule:packaging/a-package-is-its-digest` -->

# package.lock and --locked

    nvs pkg fetch --locked
    nvs pkg vendor --locked

`package.lock` lists every package your application uses. `nvs pkg fetch`, `nvs pkg vendor`,
`nvs pkg add` and `nvs pkg update` write it. Commit it with your code, and do not edit it. One entry
looks like this:

```toml
[[package]]
name = 'Acme\Slug'
version = "1.2.0"
source = "https://packages.example.com"
digest = "ad5fbbc3700675923bf1fcd46f4c46775db50fe76f05385b9d40782b75447d75"
repository = "https://git.example.com/acme/slug"
commit = "4f2a9c0e1b7d3a5f6e8c9b0a1d2e3f4a5b6c7d8e"
owners = ["acme"]

[package.capabilities]
required = []
optional = []
```

| Key | What it says |
|---|---|
| `name`, `version` | the package and the version that is used |
| `source` | the registry URL, or `git+` and the repository URL for a git dependency |
| `digest` | the BLAKE3 hash of the package's archive |
| `repository`, `commit` | where the package's source code is |
| `owners` | the accounts that may publish the package |
| `[package.capabilities]` | the capabilities the package requests |
| `hosts` | the hosts its downloaded files come from, when it has any |

Every command checks each archive against the digest in `package.lock`. When a digest does not match,
the command stops with an error and writes no file. So a changed archive or a changed digest in the
file is never used.

With `--locked`, the command also stops when `package.lock` has to change. That happens when a package
is missing from it, has another version, or is no longer needed. Use `--locked` in CI, so the build
uses exactly the versions you tested.

<!-- src: `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`, `rule:packaging/a-package-is-its-digest` -->

# nvs pkg pack

    nvs pkg pack [<dir>] [--out <file>]

`nvs pkg pack` builds the archive of the package in `<dir>`. The default is the current folder. The
archive contains only these files:

- every `.nvs` file under `src`;
- `package.toml`;
- the license, readme and notice files next to `package.toml`, such as `LICENSE`, `README.md` and
  `NOTICE`;
- each file that `[package] files` lists.

The command prints each file with its size, the total and the digest of the package:

```text
       241  LICENSE
       112  package.toml
       530  src/Slug.nvs
       883  total, 3 files
acme/slug 1.2.0  blake3:ad5fbbc3700675923bf1fcd46f4c46775db50fe76f05385b9d40782b75447d75
wrote acme-slug-1.2.0.tar
```

The same files always give the same archive and the same digest, on every system. The archive does
not contain the time, the owner or the permissions of a file.

A package must have a license file, such as `LICENSE` or `COPYING`. It has at most 5,000 files, and at most 16 MiB together. Every path must work on Windows, macOS
and Linux. A symbolic link is not allowed. For a larger file, use `nvs pkg download`.

<!-- src: `rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size` -->

# nvs pkg download and nvs pkg provide

    nvs pkg download <url> --to <path>
    nvs pkg provide <file>

A package can use a file that is too large for its archive, such as a font or a data set. Run
`nvs pkg download` in the folder of your package:

    nvs pkg download https://files.example.com/fonts/Big.otf --to fonts/Big.otf

The command downloads the file, saves it at `fonts/Big.otf` and adds this entry to `package.toml`:

```toml
[[download]]
path = "fonts/Big.otf"
urls = ["https://files.example.com/fonts/Big.otf"]
sha256 = "9f2c4e6a8b0d1f3e5a7c9b2d4f6e8a0c1b3d5f7e9a2c4b6d8f0e1a3c5b7d9f2e"
size = 734212
```

You can add more URLs to `urls`, and a `license` such as `"OFL-1.1"`. Every URL must start with
`https://`. The path may not be under `src` and may not end in `.nvs`.

When somebody fetches your package, `nvs pkg fetch` downloads each file and checks its size and its
SHA-256 hash. A file that does not match is not used, and the next URL is tried. A redirect to an
internal address, such as `127.0.0.1`, is not allowed. Nothing in the file runs. The package's own code
may read the file with no grant.

Each file is kept once in a download cache in the Novis data folder. Every project that needs the same
file uses that copy.

When no URL of a file works, get the file in another way. Then run `nvs pkg provide` with it:

    nvs pkg provide Big.otf
    nvs pkg fetch

`nvs pkg provide` adds the file to the cache only when its size and its hash match a file that a
package in `package.lock` lists.

<!-- src: `rule:packaging/the-registry-keeps-an-append-only-log` -->

# The transparency log

    nvs registry key <file> --name <name>
    nvs registry log append <dir> --key <file> --origin <name>
    nvs registry log verify <dir>
    nvs registry index sign <dir> --key <file> [--log <dir>] [--at <seconds>]

A registry keeps a log of every change it makes. Each publish, retraction, withhold, unlist and
change of owner is one entry. An entry is never changed or removed, so the log only grows. The log
uses the C2SP formats `tlog-tiles` and `signed-note`, which other transparency logs also use.

A checkpoint is the signed summary of the log. It has the log's name, its number of entries and a
SHA-256 hash of all the entries:

```text
packages.example.com
2
ivAZfe0T7wcstVA1V/8iXOr5+LHx+fw6Zb25ASoaPxU=

— packages.example.com fI4DleVohE11yxi/MdYzFqIELlvHFJqbjAVI3m1yHkn8Bk16LjuP/DWSeKcUgFTsIsrHIUvEb7l8TV2h7+phl5tdMAc=
```

The last line is an Ed25519 signature. The `key` of the registry's line under `[registries]` is the
public key that checks it. `nvs pkg` never takes the key from the registry itself.

Every `nvs pkg` command that reads a registry does these checks:

- **The log continues the one it saw last time.** `nvs pkg` keeps the newest checkpoint of each
  registry in the Novis data folder. A new checkpoint must contain every entry of the kept one.
  When it does not, the registry changed its history. `nvs pkg` stops with an error that says the
  log was forked, and it fetches nothing.
- **Each archive is in the log.** Each version in an index file names its publish entry. The entry
  must have the same package, version, digest, repository and commit. An archive that is not in
  the log is not used.
- **Each index file is signed.** `registry.json` and each package's index file are signed with
  the same key. Each one has the time it was signed and the checkpoint it was signed with. A file
  with a wrong signature, or a time more than an hour in the future, is not used. `nvs pkg` then
  tries the next file host.

`nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` also need an index file signed in the last
seven days. They choose versions, and an old file could hide a newer release or a retraction. A
build from `package.lock` and `nvs pkg fetch --locked` choose nothing, so they still work when the
registry is down.

The `nvs registry` commands are for the people who run a registry. `nvs registry key` makes the
signing key and prints the public key for `[registries]`. `nvs registry log append` reads one JSON
entry per line and signs a new checkpoint. If one entry is wrong, it adds nothing.
`nvs registry log verify` checks every file of a log from the start. `nvs registry index sign` signs
`registry.json` and every index file again. Run it at least once a day.

<!-- src: `rule:packaging/only-a-major-may-break-the-api` -->

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

<!-- src: `rule:packaging/a-publish-token-is-scoped-and-expires`, `rule:packaging/an-owner-is-notified-without-email` -->

# nvs pkg login and nvs pkg whoami

    nvs pkg login [--registry <url>]
    nvs pkg whoami [--registry <url>]

To publish, you need a publish token. You make the token on the registry's website. A token can
publish one vendor, such as `Acme`, or one package, such as `Acme\Http`. This is its scope. A
token expires after one year at most. Every token starts with `nvsreg_`.

`nvs pkg login` reads the token from standard input. Paste it when the command asks, or pipe it
in:

    nvs pkg login < token.txt

The command never takes the token as an argument, because other programs and your shell history
can see an argument. It sends the token to the registry first. If the registry does not accept it,
nothing is stored. Otherwise the command stores the token in the Novis data folder, where only your
account can read it:

```text
Logged in to `https://registry.novis-lang.org`.
Account: 3f2a9c0b7d4e4a1c9b8e2f6d0a5c7e13
Scope: Acme
Expires: 2027-10-28 (UTC)
The token is stored in `/home/ana/.nvsdata/tokens/5e0c8d4b2a7f19e3c6b1d0a4f8e2c7b9`.
```

`nvs pkg whoami` sends the stored token to the registry and prints the same account, scope and
date. It also prints each release of your vendors that is not available yet, and that you did not
confirm:

```text
Registry: https://registry.novis-lang.org
Account: 3f2a9c0b7d4e4a1c9b8e2f6d0a5c7e13
Scope: Acme
Expires: 2027-10-28 (UTC)
Releases that are not available yet:
  Acme\Http 1.4.0 will be available at 2026-11-04T09:12:00Z.
```

If you did not publish a release in this list, somebody else has a token for your vendor. Revoke
the token on the website, and retract the version. Each token works only with the registry it was
made for. `--registry` names another registry. The default is `https://registry.novis-lang.org`.

A token that a file or a log shows is easy to find, because of `nvsreg_`. Secret scanners look for
it. If a token leaks, revoke it on the website at once.

<!-- src: `rule:packaging/the-registry-builds-the-archive-from-git`, `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`, `rule:packaging/an-account-stays-active-by-proof-not-by-token` -->

# nvs pkg publish

    nvs pkg publish [--repository <url>] [--dry-run] [--registry <url>]

`nvs pkg publish` publishes the package at the last commit of its git repository. Run it in the
folder of the package. It sends no file. It sends the repository's URL, the commit, the version
from `package.toml` and the package's folder in the repository. The registry downloads that commit
and builds the archive itself, so push the commit first. Changes that are not committed are not
published.

The repository must be public, and its URL must start with `https://`. The default is the URL of
the remote `origin`. `--repository` names another one.

Before it sends anything, the command builds the archive from the same commit on your computer. It
uses the same code as the registry, so both get the same digest. `--dry-run` stops there and
prints every file:

```text
acme/http 1.4.0
Repository: https://git.example.com/acme/http.git
Commit: 5b1e0c4f9a7d2e8b3c6a1f0d4e9b8a7c6d5e4f3a
       241  LICENSE
       132  package.toml
      2106  src/Client.nvs
      2479  total, 3 files
Digest: blake3:9d2c41e07a5b3f68c1d0e4a2b7f9c3d58e6a1b0c4d7e2f9a3b5c8d1e0f4a7b6c
This was a dry run. Nothing was sent to `https://registry.novis-lang.org`.
```

Without `--dry-run`, the registry checks the package and prints what happens next:

```text
Digest: blake3:9d2c41e07a5b3f68c1d0e4a2b7f9c3d58e6a1b0c4d7e2f9a3b5c8d1e0f4a7b6c
`Acme\Http` 1.4.0 is published. It is pending until 2026-11-04T09:12:00Z. An owner can release it now with a passkey on the website.
```

A release that is sent with a token is pending for seven days. You can download a pending version
by its exact version, but `nvs pkg add` and `nvs pkg update` do not choose it. An owner who signs
in with a passkey on the website can release it at once. In the seven days, every owner of the
vendor gets a message, so a release from a stolen token can be stopped. The first version at
`0.1.0` or higher waits until an owner confirms its API with a passkey.

The registry does not publish the package when:

- the token's scope does not include the package;
- the version was published before;
- a file declares a namespace outside the package's name;
- the package has an `nvs.toml` file or a git dependency;
- a minor or a patch version breaks the API of the version before;
- the archive breaks a rule of `nvs pkg pack`;
- your account has no verified notification channel on the website;
- your account had no activity in the last 60 days.

Activity is a sign-in with a passkey, or the answer to a reminder that the registry sends to your
devices. Using a token is not activity. After 60 days with no activity, sign in with a passkey on
the website to publish again. The first publish of a package like `Acme\Http` makes your account the
owner of the vendor `Acme`. Then only its owners can publish packages under `Acme`.

If no owner of the vendor has a working notification channel, the command prints a warning. If a
publish fails while the registry writes its files, publish the same commit again.

<!-- src: `rule:packaging/a-known-bad-version-is-retracted-not-deleted`, `rule:packaging/an-owner-may-unlist` -->

# nvs pkg retract and nvs pkg unlist

    nvs pkg retract <version> --reason <text> [--fixed-in <version>] [--registry <url>]
    nvs pkg unlist [<version>] [--undo] [--registry <url>]

Run these commands in the folder of your package. A version is never deleted. A retracted or
unlisted version can still be downloaded, so a `package.lock` that names it still works.

`nvs pkg retract` marks a version as broken or unsafe. `nvs pkg add` and `nvs pkg update` do not
choose a retracted version. The package index shows the reason, which has at most 500 characters.
`--fixed-in` names the version that fixes the problem:

    nvs pkg retract 1.4.0 --reason "Client::send drops the last header." --fixed-in 1.4.1

With a token, the retraction starts seven days later. Every owner gets a message, and an owner can
make it start at once with a passkey on the website:

```text
Version 1.4.0 is retracted from 2026-11-04T09:12:00Z. An owner can retract it now with a passkey on the website.
```

`nvs pkg unlist` hides a package, or one version of it, from search and listings. Without a
version, every version is hidden. `--undo` shows it again. Only an owner who signs in with a passkey
can unlist. With a token, the command prints the page where an owner confirms it:

```text
To unlist it, sign in with a passkey and confirm on https://registry.novis-lang.org/packages/acme/http.
```

When the registry saved the change but could not update the package index yet, the command also
prints this line. The registry updates the index on its next run.

```text
The registry saved the change. It updates the package index on its next run.
```

The people who run the registry cannot delete or change a version. They can only withhold a version
that contains malware, or that a court orders them to remove. A withheld version cannot be
downloaded, and the package index shows the reason.
