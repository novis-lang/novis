`autoload` is a top-level declaration in two forms, both taking **literal strings only**:

```nvs
autoload 'Framework' from './';                        // one prefix, one root
autoload 'Acme\Legacy' from '../vendor/acme/lib',
                            '../vendor/acme/compat';   // one prefix, several roots
autoload discover '../../*/src';                       // each directory names its own prefix
```

Paths resolve **relative to the file that declares them**, never to the entry point, which is what lets
one framework directory serve many unrelated project trees. A concatenated or interpolated path is
`E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY`, reported by the parser, the only place the difference is still
visible. A prefix carries no trailing separator; matching appends one. A single-quoted `'Acme\Legacy\'`
could not be written anyway: its `\'` escapes the quote and swallows the rest of the line.

Because the map is source, it travels with the code: it resolves identically under `nvs run`,
`nvs serve`, `nvs build --compile`, the wasm target and an editor's `nvs check`, none of which share a
deployment configuration, and there is no configuration file under a document root to reach.

Every prefix segment is a `PascalCase` namespace segment, or the declaration is
`E_AUTOLOAD_PREFIX_SHAPE`: a prefix that no name can match, such as the PSR-4 `'App\*'`, is reported
where it is written. One segment may instead be `{` + a path of `.` and `..` steps + `}`, which is
replaced while compiling by the name of the directory those steps reach from the declaring file, as
the disk spells it — `autoload 'App\{..}' from '../src';` in `Blog/public/index.nvs` is
`autoload 'App\Blog' from '../src';`. That name is held to the same shape, and a prefix takes one such
segment at most. This is how many modules share one entry file byte for byte while each maps only its
own root.

The braced segment is resolved while the map is built, so the map, `implementing<T>()` and
`--autoload-map` see only the result. It produces a namespace segment held to the same test as any
other, never a path, so it opens no traversal. Where a program writes it, the program's meaning
depends on a directory's name: renaming the directory renames the namespace, and every class under it
then fails `E_AUTOLOAD_FILE_SHAPE` (`rule:programs/one-declaration-per-autoloaded-file`). The compile
cache keys a unit on its entry's path, so identical entry files in two directories never share a unit.

A root that does not exist is allowed, because a deployment leaves a module out by not shipping its
directory, and `nvs check --autoload-map` lists it under a counted `missing` section, canonical as far
as the disk allows so it prints beside the others.

`discover` takes a glob containing exactly one `*` occupying a whole path segment. Every directory it
matches becomes a root and the matched segment becomes that root's prefix. A matched directory whose
name is not a legal `PascalCase` namespace segment is **skipped, not diagnosed** — a glob over a
filesystem inevitably sweeps `.git` and `vendor`. The glob itself is held stricter: one that is not a
single whole-segment `*`, and one whose base directory does not exist, are both
`E_AUTOLOAD_GLOB_SHAPE`, because a typo that silently discovers nothing is the worst outcome on offer.

Longest matching prefix wins; within a prefix, roots are probed in declaration order and the first hit
wins, which is what makes a vendor override work. Remaining segments are directories, the last is the
file name plus `.nvs`, and the on-disk entry is compared exactly (`rule:programs/path-case`). An
explicit prefix beats a `discover` glob producing the same prefix and the glob skips that name. Two
explicit declarations whose roots resolve to the same directories in the same order are one home, so
an application's request entry and its scheduled scripts each write the line and `nvs check` over the
folder reads them together. Any other duplicate is `E_DUPLICATE_AUTOLOAD_PREFIX`, naming both sites.

A miss is not a diagnostic of its own: the name stays undeclared, and the site that named it is
`E_UNDEFINED_CLASS`. When the program declares any `autoload`, that diagnostic carries a note naming
where resolution looked: every file probed for a name under a prefix, in probe order, and the prefixes
there are for a name under none. A wrong root then reads differently from a typo in the class name.
The note is built on a failed compile only, with one probe per root, through the plain resolve, so
the cache key's probe trace does not move (`nvs_hir::AutoloadMap::note_searched_roots`).

Declarations are honoured only in a file reachable through `require` from the entry point. One inside
an autoloaded file is `E_AUTOLOAD_IN_AUTOLOADED_FILE`, because the map would otherwise depend on
itself; operationally the map is fixed the moment it is first consulted, so the rule reaches the whole
autoloaded sub-graph. Within the bootstrap chain the effective map is the **union** of every
declaration, which with the duplicate rule makes it order-independent.

Path traversal is structurally impossible, with no sanitizer: a resolved suffix is built only from
namespace segments, which are `PascalCase` identifiers that may not begin with `_`, so `.`, `..` and a
path separator cannot occur in one.

A hard cross-module reference requires that module while compiling: `Blog\Thing extends Shop\Base` does
not build if `Shop` is not deployed, where PHP fails only if the line is reached. Modules that depend on
interfaces the framework owns, rather than on each other's concrete classes, are unaffected.

- **Memory:** the effective map is O(prefixes) per host, a few hundred bytes, and nothing of it is held
  per request. A braced segment costs one canonicalization while compiling and nothing at run time.

**Declined:**
- **A manifest file found by walking up from the entry file.** It lands inside or beside a document
  root, and a walk-up finds the wrong root when many project trees share one framework directory. It
  would also need a format, a discovery rule, an `extends` key and path anchors that a declaration in
  the framework's own bootstrap file gets for free (`rule:config/nvs-toml-is-not-a-project-manifest`).
- **An `[autoload]` table in `nvs.toml`.** The map is a property of the source tree, not of the
  deployment (`rule:config/nvs-toml-is-not-a-project-manifest`).
- **A `declare()`-style pragma.** Novis has no `declare()` grammar to extend, and a top-level statement
  shaped like `require` already fits.
- **Convention only, the entry file's directory as the global namespace root.** It cannot express
  `src/` beside `tests/` beside `vendor/`, several roots for one prefix, or a framework outside the
  project tree.
- **A generated classmap.** A build step producing a file that goes stale is strictly worse than
  resolving from the source the compiler already reads.
- **A PSR-4 wildcard, `'App\*' from '../../*/src'`, to share one entry file.** It maps every module
  into every program, so each module's `#[Route]` table and `implementing<T>()` hold every other
  module's classes, and two modules that both declare `/` collide. It would also need roots that
  resolve by name without being enumerated, and still a way to name the current module.
- **`discover` taking a prefix.** It maps every directory the glob matches, which is the same problem.
- **`*` instead of `{..}` inside a prefix.** A reader coming from PSR-4 reads `*` as a wildcard, and
  that is the meaning the braced segment does not have.
- **A missing root as an error.** A deployment leaves a module out by leaving its directory out.
