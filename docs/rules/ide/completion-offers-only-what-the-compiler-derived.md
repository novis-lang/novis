`nvs-lsp` contains no framework-specific module, no annotation dialect, no scan for a framework's
conventions and no directory-layout knowledge. It offers a **value** in a completion list only where the
compiler already derives that value for another reason, reaching it through the same table that other
reason uses, or where a completion file under a `.novis/completion/` folder lists it for the parameter
the cursor is at (`rule:ide/completion-files-offer-values-at-named-parameters`). This is the whole of
Novis's answer to "framework support", and it is a closed rule, not a starting point.

What that admits: the methods that carry a `#[Route]`, offered first in a link's or a match
check's first argument, and their parameters, from the route table
`rule:routing/routes-are-compiled-not-registered` builds while compiling — the same table
`rule:routing/link-target-and-params-are-checked` checks a link against; configuration directives in a
file named `nvs.toml` (`rule:ide/the-extension-claims-nvs-only`), from the default file the binary ships,
which `bun nv directives --check-template` holds to the parser that refuses an unknown key
(`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`), so completion, hover with type and
default, and "no such directive" are three readings of one registry, and `[[include]]`'s `path` and `dir`
complete as paths — scoped to the workspace's config tree, never to every TOML file; a `require` path,
an `autoload` root or `discover` glob, and a string argument at a parameter the registry or
`#[Core\Path]` marks as a path, which complete as the entries of the one directory the text before the
cursor reaches, read through the listing the compiler resolves a glob with; an `autoload` prefix, which
completes as the namespaces of the declarations the workspace index and the analysis already hold; the
string converted with `as class<T>`, which completes as the classes the program declares that are a
`T`, read through the hierarchy walk the checker answers that question with; a string argument at a
parameter the registry marks as a class name, which completes as the classes the program declares, and
as the ones that are a `Throwable` where the member expects an error; a string argument at a parameter
whose declared type is a set of allowed string values (`rule:types/single-value-types`), which completes as
those values, read off the parameter types the checker records on the call it resolved; the string
converted with `as property<T>`, and a string argument at a parameter whose type takes a `property<T>`
beside a `string`, which complete as `T`'s public declared properties with their types, read off the set
the checker tests that conversion against (`rule:types/property-key`); the field names of a compiler
attribute's payload, such as `#[Api]`'s, from the attribute's card, which is held to the roster the
checker reads the payload against; the security schemes the program declares with `#[Api\Scheme]`, at
an entry of an `#[Api]` `security` list, from the route table the checker holds each such name against
(`rule:routing/a-security-scheme-is-declared-in-code`); every attribute's anonymous object, whose shape
is a declared type; and enum cases, members off a resolved receiver
and in-scope variables, which are the same rule and not an exception to it.

A path list is one directory: the text between the opening quote and the cursor is split at its last
separator, the part before it is joined to the declaring file's directory (the base `require` and
`autoload` resolve against) or replaces it when absolute, and that directory is listed through
`nvs_hir::autoload::entries_of`, so a name offered is one resolution would find. Names are offered as the
disk spells them (`rule:programs/path-case`), a name starting with `.` is hidden, and an item replaces
only the segment after the last separator. A directory item writes a `/` and opens the list again. A
`require` offers directories and `.nvs` files, an `autoload` literal directories only, and a path
argument every file, since a path parameter takes a file of any kind. Nothing is offered once the
directory part holds a `*`. An `autoload` prefix offers namespaces from the workspace index as well as
the analysis, `Core` excluded, because a prefix usually maps code nothing loads yet. The string of `as
class<T>` offers each class and interface in the analysis that `nvs_hir::hierarchy::implements_interface`
says is a `T`, `T` included, and not a class only an autoload root could load, because the run-time
conversion compares whole names among the classes the program declares and cannot find that one either.
A name literal's item is a whole name replacing all the text after the opening quote: a `\` typed just
before the closing quote escapes it, so no list can be answered at that keystroke, and a whole name
never needs it. `Core\Test::assertThrows` is the one member whose class-name parameter expects an
error, named in a one-row table in `crate::arguments`, since the mark says only that the text is a class
name. A parameter whose type is a set of allowed string values completes them as items of kind
`value`, its `null` aside, and is offered a completion file's list beside them, merged by value; it gets
no warning for a literal outside the set, because the checker already rejects one.

A property name is an item of kind `property` with its declared type at the right. The checker records
the set for each class a `property<T>` names at an `as` conversion, and at a string literal argument for
a parameter whose type has a `property<T>` member, because the walk that makes the set needs the signature
table and the editor does not keep one. A parameter typed `property<T>|string` takes any string, so a
literal outside the set gets no warning there. A parameter typed `property<T>` alone, or `?property<T>`,
is offered the same names, because the checker converts a string literal there while compiling
(`rule:types/property-key`), and a wrong one is its own diagnostic. A parameter typed `propertyPath<T>`,
and a hole of a literal at a `string<K>` parameter or under `as string<K>`, is offered the names one
segment at a time, from the sets the checker records for each class the path reaches
(`rule:ide/a-field-path-completes-one-segment-at-a-time`). A hole of a `string<K>` whose `K` lists
string values is offered those values.

- **Memory:** one list of names and types per class a `property<T>` names or a path reaches, kept with
  the checker's table for the length of the analysis. Nothing at run time.

What it refuses has no subject rather than being declined: ORM columns (a codec's fields are declared
properties, already reached as members), and service-container, facade and view-name resolution (`Core`
has no container and no view layer, `rule:programs/no-first-party-framework`). A vendor annotation dialect is
refused outright as a second, unchecked description of the program's shape — the failure
`rule:routing/api-document-is-generated-from-the-route-table` refuses for API documents. A completion
file is not one: it describes values the compiler never holds, such as the keys of a dataset or the
names in an icon set, and never a class, a member or a type. It is data and is never run. And **the
language server makes no network request**: a lockfile on disk may be read, a remote index may not be
consulted.

A test, not review, enforces this: `nvs-lsp`'s completion sources are enumerated, and each must name a
table the compiler builds for another reason or the table the completion files were loaded into. No
completion source reads a directory or a file itself.

The text an item inserts comes from the same place: a `Core` member is inserted from the registry the
checker seeds its signatures from, so a suggestion never writes a call that then fails to resolve. An
editor that writes such a call is worse than one that offers nothing, because the developer cannot tell
the suggestion's mistake from their own.

The rule buys a stronger guarantee than framework-aware completion elsewhere gives. A route's
method completed from the compile-time route table is correct, where one recovered by scanning a conventions
directory is a good guess. Columns recovered from migrations, facades followed through a runtime
container and components found by directory convention are each a heuristic replacing a fact the
language does not make available; Novis makes those facts available, so the heuristic has nothing to
replace. Stating the rule as a property of where a value comes from keeps `nvs-lsp` free of
per-framework modules, which would otherwise arrive one framework at a time and never leave.

What it costs: a package whose route-like table is built while the program runs gets no completion for
it, and the answer is to make that table compile-time. An editor that talks to a third party while a
developer types is the same class of surprise `rule:http-server/allow-url-pins-the-address` refuses for
a running program, which is why no completion source fetches anything.

**Declined:**
- **Framework support built from convention scans, an annotation dialect and a patch file** for what
  neither reaches. Each is a second description of a program's shape that nothing checks against the
  first.
- **A module for the first-party web framework inside `nvs-lsp`.** It would be the server's first
  special case, and a third-party package would then ask for the same treatment with no principle left
  to refuse it.
- **Package-name and version completion in the manifest, from a registry.** It needs a network request
  while the user types. A lockfile's resolved versions are on disk and may be shown.
- **PHP built-in names as candidates, each answered with its Novis destination.** The answer needs a
  table from PHP names to Novis ones, and there is none (`rule:programs/no-compatibility-promise`).
- **A PHP name that resolves while the program runs, as an alias of a `Core` member.** It is a second
  name for one member (`rule:statements/nothing-gets-a-second-name`), and it keeps PHP names in
  committed source for good.
- **A `std::fs` listing in `completion.rs`.** It would bypass the test that keeps every completion
  source on a table the compiler builds, and could disagree with what resolution finds.
- **A completion file that overrides a method's return type.** The editor's types would then disagree
  with the compiler's, which is the second description of the program's shape this rule refuses.
- **Offering every workspace class for the string of `as class<T>`.** A class the program never loads
  is one the conversion throws for.
- **A bound in the class-name mark, so `assertThrows` would carry `Throwable` in the registry.** One
  member needs it, and a new mark field would be read by nothing but the editor.
