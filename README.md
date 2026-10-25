# Novis-Lang

**The programming language for web applications and the command line. Fast. Secure by default.**

You can build a business app, a website, an API or any other web service with it. The tools that a
modern web application needs are built in.

> **Status: Pre-Alpha.** Novis is released, and the [install page](https://novis-lang.org/install/) has
> the latest version. Any release can change the language, and Novis is not ready for production use.

Free and open source · Windows, Linux and macOS · An extension for VS Code · Ready for Docker

[Install Novis](https://novis-lang.org/install/) · [Take the tour](https://novis-lang.org/guides/tour/) ·
[novis-lang.org](https://novis-lang.org/)

- **[Input is tainted.](https://novis-lang.org/in-depth/concepts/tainted-input/)** Text from a request
  must be checked or escaped before it reaches a database, a shell or a page.
- **[Every request on its own.](https://novis-lang.org/in-depth/concepts/isolation/)** Each request has
  its own memory. When one request crashes, the others keep running.
- **[One program.](https://novis-lang.org/reference/cli/one-binary/)** `nvs` runs your code, serves web
  requests, runs your tests and formats your files.
- **[Compiled to machine code.](https://novis-lang.org/in-depth/concepts/compiled/)** Each file is
  compiled the first time it is needed. You never run a build step.

## The compiler finds the mistake first

Some mistakes are common in web code. The Novis compiler finds them before any of your code runs.

- **Text from a request in a database query.** A value from a request is a `tainted string`. It cannot
  become part of the text of a query. You send it as a parameter instead.
  [How tainted input works](https://novis-lang.org/guides/why-novis/tainted-input/)
- **A password in the output.** A password or a key is a `secret string`. Code that prints it or writes
  it to a log does not compile. [How secret values work](https://novis-lang.org/guides/why-novis/secret-values/)

## Small programs, clear rules

Novis checks the things that often go wrong in web code. The checks are part of the language, so there
is nothing extra to install.

- **[Every conversion is checked.](https://novis-lang.org/guides/why-novis/checked-conversions/)**
  `as int` throws an error when the text is not a whole number. `as ?int` gives `null`, and your code
  decides what happens next.
- **[Text as people read it.](https://novis-lang.org/guides/why-novis/unicode-strings/)** `Core\Str`
  counts the characters a person sees. An accent or an emoji stays whole.
- **[JSON in one line.](https://novis-lang.org/reference/core/json/)** `Core\Json::encode` writes a
  shape, a list, a string or any other value as JSON. A shape keeps its keys in the order you write them.
- **[Jobs that run at the same time.](https://novis-lang.org/syntax/concurrency/tasks/)**
  `Core\Task::all` runs several functions at the same time. Each result has its own name and its own type.
- **[HTML that escapes itself.](https://novis-lang.org/guides/why-novis/html-escapes/)** An `html`
  template escapes every value in it. A comment from a user is shown as text.
- **[Typed JSON.](https://novis-lang.org/guides/why-novis/typed-json/)** `Core\Json::decodeAs` reads
  JSON into an object of your class and checks every property.
- **[Null is checked.](https://novis-lang.org/guides/why-novis/no-null-surprises/)** A `User` is never
  `null`. Only a `?User` can be. The compiler checks every place that uses it before the program runs.
- **[Errors with a default.](https://novis-lang.org/guides/why-novis/simple-errors/)** `catch` after a
  call gives a default value when the call throws an error.

## One request cannot break another

A request can crash, run out of memory or loop forever. When that happens, only that request stops. The
server keeps running, and every other request finishes normally.

- Each request has its own memory and its own copy of every static property.
- You set limits for memory and time in `nvs.toml`. They apply to every request.
- A request that uses more than a limit stops immediately. Only that request stops.

[How isolation works](https://novis-lang.org/in-depth/concepts/isolation/) ·
[Memory and CPU limits](https://novis-lang.org/in-depth/concepts/limits/)

## One library comes with it

It has classes for JSON, databases, HTTP, passwords, dates and much more. Most programs need no package
from outside. Every class has its own page in the [reference](https://novis-lang.org/reference/). It
lists each method and its parameters.

## One program runs, tests and serves your code

`nvs` is one program. It runs your code, serves web requests, runs your tests and formats your files.

| Command | What it does |
|---|---|
| [`nvs run app.nvs`](https://novis-lang.org/reference/cli/nvs-run/) | Check, compile and run a program |
| [`nvs serve app.nvs`](https://novis-lang.org/reference/cli/nvs-serve/) | Serve web requests |
| [`nvs test tests/`](https://novis-lang.org/reference/cli/nvs-test/) | Run every test |
| [`nvs check app.nvs`](https://novis-lang.org/reference/cli/nvs-check/) | Report every error and run nothing |
| `nvs fmt` | Format your files |
| [`nvs build --compile app.nvs`](https://novis-lang.org/reference/cli/nvs-build-compile/) | Write one executable file with your program |
| [`nvs build --openapi app.nvs`](https://novis-lang.org/reference/cli/nvs-build-openapi/) | Write the OpenAPI document of your web API |
| [`nvs lsp`](https://novis-lang.org/install/editor/) | Errors and completion in your editor |

Tests live next to your code. A test is a method with `#[Test]`. `nvs test` runs each test on its own.
You do not install a package for it. [Tests in the language](https://novis-lang.org/guides/why-novis/tests-in-code/)

## Let a coding agent write your Novis code

`nvs` contains the full documentation of the language and its library. A coding agent looks up what it
needs. The compiler checks every file the agent writes before the file runs.

- **The documentation is in `nvs`.** `nvs agent find` and `nvs agent show` print the documentation of
  each class, method, setting and error code. It always matches the version you installed.
- **One command sets up your project.** `nvs agent init` writes an `AGENTS.md` file and the files your
  agent reads.
- **Errors after every edit.** In Claude Code and Cursor, `nvs` checks each `.nvs` file after the agent
  edits it. The agent sees the errors at once.

[Set up your project](https://novis-lang.org/reference/cli/nvs-agent-init/) ·
[All agent commands](https://novis-lang.org/reference/cli/nvs-agent/)

## Measured and counted

The [home page](https://novis-lang.org/) shows the numbers of the latest recorded benchmark run. It also
counts the tests in the repository and the crates with no unsafe Rust.
[Compare with other languages](https://novis-lang.org/in-depth/compare/) ·
[How Novis is tested](https://novis-lang.org/in-depth/how-novis-is-written/#tested-from-every-direction)

## Roadmap

This is the work that is planned, in the order it is planned. The order can change, and there are no
dates. The [roadmap page](https://novis-lang.org/in-depth/roadmap/) has the same list.

<!-- roadmap:start (bun nv site --roadmap writes this list from website/src/data/roadmap.json) -->
1. **Editor.** *Working on it now.* The editor can rename, find, import and generate code, and it has more quick fixes. It can also show what your program did for a recorded request.
2. **Web APIs.** The API document that Novis writes for your server passes a strict OpenAPI check. A parameter can read its value from the query string. A wrong value gives a `400` error.
3. **Debugging and profiling.** A debugger stops your program at a line and shows its variables, in VS Code too. Novis records requests, and a viewer shows each one with its timeline, logs and errors. The profiler shows where the time goes.
4. **Images.** AVIF and JPEG XL images open the right way up and with the right colours. `Image::info` reads EXIF data, and an animated image keeps every frame.
5. **Packages and the registry.** `nvs` installs and locks the packages your program uses, and the registry builds every package from its source code. `nvs pkg audit` lists the known security problems in your packages, and the version that fixes each one.
6. **Passkeys and Web Push.** `Core\Passkey` lets a user sign in with a passkey. `Core\WebPush` sends a notification to a browser.
<!-- roadmap:end -->

## Contributing

Ideas, questions and disagreements go to [Discussions](https://github.com/novis-lang/novis/discussions).
Bugs and concrete proposals go to [Issues](https://github.com/novis-lang/novis/issues). Talk with us
every day on [Discord](https://discord.gg/8ftMjPeH8h). All of these are open to anyone, at any time.

**Pull requests are not open yet.** The core team builds the language to one design first. When the
design is settled, pull requests open. Then the community and the core team maintain Novis together.
This affects code contributions only.

[CONTRIBUTING.md](CONTRIBUTING.md) says where each kind of contribution goes. Its second half is for
developers. It describes the design, the folders of the repository, how to build and test, and the
conventions for a change. Then read [AGENTS.md](AGENTS.md). It lists the priorities for every design
decision and the rules that are easy to break. It also has a table of which document to read for each
kind of work.

## Sponsoring

Novis is open source and free to use. It is under the MIT license, and it always will be. A solid
language needs time, and infrastructure that costs money every month. Sponsors pay for this work.

You can give through GitHub Sponsors, Polar or a bank transfer. You can give once or every month. All
money, after the fees of the platforms, goes into development, infrastructure and the ecosystem. Every
quarter, we publish a short report with the number of donors, the total and what we spent it on.

By default, we publish nothing about you: not your name, not your logo and not your amount. These
promises stay the same:

- Novis is open source under the MIT license, and it always will be.
- The open-source release is all of Novis. There are no paid features and no hidden parts of the core.
- Everybody can learn Novis, build with it and deploy it for free.

The [sponsoring page](https://novis-lang.org/sponsoring/) lists the tiers, the ways to give and the
reports. If you cannot give money, your ideas and questions help too. [CONTRIBUTING.md](CONTRIBUTING.md)
says where they go.

## License

Novis is under the [MIT](LICENSE) license. Contributions are accepted under the same license.

Every third-party component in the `nvs` binary has a permissive license.
[THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt) lists each one with its copyright notice and its full
license text. A tool generates this file from the dependencies. The file is also built into the binary,
so every copy of `nvs` contains the notices:

```sh
nvs info                # facts about the build and the host, and every component with its license
nvs info --licenses     # the same, and the full text of every license

bun nv gen-attribution           # generate the file again after you change a dependency
bun nv gen-attribution --check   # what CI runs: it fails if the file is out of date
```

The generator stops the build for a license it has no policy for. It also stops it for a license that is
not in the allow list of [deny.toml](deny.toml). So a notice is never left out. `cargo deny` checks which
licenses may be linked. The generator writes the notices that must ship with the binary.
