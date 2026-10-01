`nvs-lsp` contains no framework-specific module, no annotation dialect, no scan for a framework's
conventions and no directory-layout knowledge. It offers a **value** in a completion list only where the
compiler already derives that value for another reason, reaching it through the same table that other
reason uses, or where a completion file under a `.novis/completion/` folder lists it for the parameter
the cursor is at (`rule:ide/completion-files-offer-values-at-named-parameters`). This is the whole of
Novis's answer to "framework support", and it is a closed rule, not a starting point.

What that admits: route names and their parameters, from the route table
`rule:routing/routes-are-compiled-not-registered` builds while compiling — the same table
`rule:routing/link-name-and-params-are-checked` checks a link against; configuration directives in
`nvs.toml` and every file `[[include]]` pulls in, from the closed registry the runtime validates against
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
as the ones that are a `Throwable` where the member expects an error; `#[Api]` fields and
every attribute's shape literal, which is a declared type; and enum cases, members off a resolved receiver
and in-scope variables, which are the same rule and not an exception to it.

What it refuses has no subject rather than being declined: ORM columns (a codec's fields are declared
properties, already reached as members), service-container and facade resolution (constructor injection
is resolved while compiling, so go-to-definition already goes there), and view-name completion
(`rule:programs/first-party-framework` makes the view layer the language). A vendor annotation dialect is
refused outright as a second, unchecked description of the program's shape — the failure
`rule:routing/api-document-is-generated-from-the-route-table` refuses for API documents. A completion
file is not one: it describes values the compiler never holds, such as the keys of a dataset or the
names in an icon set, and never a class, a member or a type. It is data and is never run. And **the
language server makes no network request**: a lockfile on disk may be read, a remote index may not be
consulted.

One thing offered is not a *value*: a PHP built-in's name, admitted as a candidate from an audited table
and bounded on the insert side by `rule:ide/three-of-four-item-shapes-insert-nothing`.

A test, not review, enforces this: `nvs-lsp`'s completion sources are enumerated, and each must name a
table the compiler builds for another reason or the table the completion files were loaded into. No
completion source reads a directory or a file itself.
