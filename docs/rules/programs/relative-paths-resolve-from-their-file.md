A relative string literal passed to a path parameter is joined to the folder of the source file that
contains it, while compiling, the way `require` resolves its path. The program runs with the absolute
path in the string's place, so `Core\IO::read('data/x.json')` reads the same file from a terminal,
under `nvs serve` and as a service.

```nvs
// src/Report.nvs
Core\IO::read('data/rates.json');        // -> <folder of src/Report.nvs>/data/rates.json
Core\IO::read('/srv/shop/rates.json');   // absolute: unchanged
```

**A path parameter** is one whose text is marked as a path. `Core` marks every parameter that names a
file on disk — `Core\IO`'s path members, `Core\Process::run`/`spawn`, `Core\Response::sendFile`,
`Core\Http\Part::file`, both `saveTo` members, `Core\Net::connectLocal`/`listenLocal`,
`Core\Zip::extract`'s destination and `Db\Settings`' `path`. The script an isolate runs is a path
too: the operand of `spawn script` and the entry of `Core\Socket::upgrade` and `Core\Sse::upgrade`,
when it is a path and not a static method, and `Core\Queue::push`'s script, which the job's row
stores absolute. `Core\Path`'s own members work on path
text and are not marked, and neither is `Core\IO::within`'s `$path`, which names a file under its
`$base`. A method marks its own parameter with `#[Core\Path]`, which takes no payload
and is allowed only on a parameter whose type is a `string`, alone or with `null` (`E0836`
elsewhere). A default value of such a parameter resolves against the file that declares it.

**A string literal** here is one written as the argument itself. A class constant, a
concatenation of string literals and a variable are values built while the program runs. A path
argument that starts with a relative string literal and adds more — `'data/' . $name` or `"data/{$name}"` — is
relative on every run, so it does not compile (`E0840`); `Core\Path::thisDir('data') . '/' . $name`
builds it from the file's folder. A single letter, which a `:` could turn into a drive, and a
heredoc are left to the run-time check. The join is
lexical: `.` and `..` are removed from the text and nothing on disk is read, so the answer does not
depend on whether the file exists yet. The capability check still canonicalizes the absolute path
before it compares it with a grant (`rule:security/path-scope-canonicalise-then-prefix`). Inside a
bundled executable the file's folder maps to the same folder beside the executable.

**A program names its own file and folder with `Core\Path::thisFile()` and `Core\Path::thisDir()`.**
The compiler replaces each call with the absolute path of the file that contains it, or that file's
folder, from the same folder a string literal is joined to, and the program runs a string constant.
`thisDir($join)` adds a relative string literal to the folder, lexically; any other `$join` does not
compile (`E0837`), and `Core\Path::join(Core\Path::thisDir(), $part)` joins a path the program
builds. There are no magic constants: `__FILE__` and `__DIR__` stay `E0319`.

**A relative path that reaches a `Core` door at run time throws** a `RuntimeError` naming the member
and the path, before any grant is asked. `Core\Path::join` builds an absolute path from a folder the
program holds, and `Core\Path::fromCwd` joins a path typed on a command line to the working directory.
`fromCwd` throws while a request is being answered, because a server's working directory is not the
app's. Its answer is a plain `string` without `tainted`, so a path from `Core\Cli::arguments` reaches
a path door through it (`rule:security/launderers-are-sink-named` states the exception). The resolver an isolate's script goes through refuses a relative path the same way, for a
spawn, a queued job, a scheduled entry and a log handler alike. With every path absolute at the check, a relative grant in `nvs.toml`, which resolves against
that file's folder, names the same files as a string literal in a program beside it.
