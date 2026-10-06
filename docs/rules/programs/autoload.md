`autoload` is a top-level declaration in two forms, both taking **literal strings only**:

```nvs
autoload 'Framework' from './';                        // one prefix, one root
autoload 'Acme\Legacy' from '../vendor/acme/lib',
                            '../vendor/acme/compat';   // one prefix, several roots
autoload discover '../../*/src';                       // each directory names its own prefix
```

Paths resolve **relative to the file that declares them**, never to the entry point, which is what lets
one framework directory serve many unrelated project trees. A concatenated or interpolated path is
`E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY`. A prefix carries no trailing separator; matching appends one.

Every prefix segment is a `PascalCase` namespace segment, or the declaration is
`E_AUTOLOAD_PREFIX_SHAPE`: a prefix that no name can match, such as the PSR-4 `'App\*'`, is reported
where it is written. One segment may instead be `{` + a path of `.` and `..` steps + `}`, which is
replaced while compiling by the name of the directory those steps reach from the declaring file, as
the disk spells it — `autoload 'App\{..}' from '../src';` in `Blog/public/index.nvs` is
`autoload 'App\Blog' from '../src';`. That name is held to the same shape, and a prefix takes one such
segment at most. This is how many modules share one entry file byte for byte while each maps only its
own root.

A root that does not exist is allowed, because a deployment leaves a module out by not shipping its
directory, and `nvs check --autoload-map` lists it under `missing`.

`discover` takes a glob containing exactly one `*` occupying a whole path segment. Every directory it
matches becomes a root and the matched segment becomes that root's prefix. A matched directory whose
name is not a legal `PascalCase` namespace segment is **skipped, not diagnosed** — a glob over a
filesystem inevitably sweeps `.git` and `vendor`. The glob itself is held stricter: one that is not a
single whole-segment `*`, and one whose base directory does not exist, are both
`E_AUTOLOAD_GLOB_SHAPE`, because a typo that silently discovers nothing is the worst outcome on offer.

Longest matching prefix wins; within a prefix, roots are probed in declaration order and the first hit
wins, which is what makes a vendor override work. Remaining segments are directories, the last is the
file name plus `.nvs`, and the on-disk entry is compared exactly (`rule:programs/path-case`). An
explicit prefix beats a `discover` glob producing the same prefix and the glob skips that name; any
other duplicate is `E_DUPLICATE_AUTOLOAD_PREFIX`, naming both sites.

Declarations are honoured only in a file reachable through `require` from the entry point. One inside
an autoloaded file is `E_AUTOLOAD_IN_AUTOLOADED_FILE`, because the map would otherwise depend on
itself; operationally the map is fixed the moment it is first consulted, so the rule reaches the whole
autoloaded sub-graph. Within the bootstrap chain the effective map is the **union** of every
declaration, which with the duplicate rule makes it order-independent.

Path traversal is structurally impossible, with no sanitizer: a resolved suffix is built only from
namespace segments, which are `PascalCase` identifiers that may not begin with `_`, so `.`, `..` and a
path separator cannot occur in one.
