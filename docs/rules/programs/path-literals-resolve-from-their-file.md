A relative string literal passed to a path parameter is joined to the folder of the source file that
contains it, while compiling, the way `require` resolves its path. The program runs with the absolute
path in the literal's place, so `Core\IO::read('data/x.json')` reads the same file from a terminal,
under `nvs serve` and as a service.

```nvs
// src/Report.nvs
Core\IO::read('data/rates.json');        // -> <folder of src/Report.nvs>/data/rates.json
Core\IO::read('/srv/shop/rates.json');   // absolute: unchanged
```

**A path parameter** is one whose text is marked as a path. `Core` marks every parameter that names a
file on disk — `Core\IO`'s path members, `Core\Process::run`/`spawn`, `Core\Response::sendFile`,
`Core\Http\Part::file`, both `saveTo` members, `Core\Net::connectLocal`/`listenLocal`,
`Core\Zip::extract`'s destination and `Db\Settings`' `path`. `Core\Path`'s own members work on path
text and are not marked. A method marks its own parameter with `#[Core\Path]`, which takes no payload
and is allowed only on a parameter whose type is a `string`, alone or with `null` (`E0836`
elsewhere). A default value of such a parameter resolves against the file that declares it.

**A literal** is a plain string literal written as the argument itself. A class constant, a
concatenation of literals and a variable are values built while the program runs. The join is
lexical: `.` and `..` are removed from the text and nothing on disk is read, so the answer does not
depend on whether the file exists yet. The capability check still canonicalizes the absolute path
before it compares it with a grant (`rule:security/path-scope-canonicalise-then-prefix`). Inside a
bundled executable the file's folder maps to the same folder beside the executable.

**A relative path that reaches a `Core` door at run time throws** a `RuntimeError` naming the member
and the path, before any grant is asked. `Core\Path::join` builds an absolute path from a folder the
program holds, and `Core\Path::fromCwd` joins a path typed on a command line to the working directory.
`fromCwd` throws while a request is being answered, because a server's working directory is not the
app's. With every path absolute at the check, a relative grant in `nvs.toml`, which resolves against
that file's folder, names the same files as a literal in a program beside it.
