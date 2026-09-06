# Architecture Decision Records

Each ADR records a decision that would be expensive to reverse, and *why* — so that a future reader can
tell a deliberate trade-off from an accident. Add one whenever a choice constrains later work.

Files get their own document only when the reasoning is subtle or contested. Decisions that are simply
recorded, with no live tension worth arguing, live in *Decisions taken at project start* below.

**Reading one.** Every ADR opens with a metadata block and an **In short** paragraph carrying the whole
decision. If you only need the rule, stop there. `Context`, `Alternatives rejected` and `Revisiting` exist
for when you intend to *change* the decision. Numbering starts at 0002 and has gaps where an ADR was folded
into another; the project-start section below is what 0001 would have been.

**The metadata block is a closed field set**, and every field in it is one of `Status`, `Date`,
`Scope`, `Depends on`, `Amends`, `Amended by`, `Validated by`. `Status` is a bare value, never a
paragraph. `Amends:` is one clause per target saying what changed there; `Amended by:` is bare numbers
and nothing else, because README's own fold rule is what an explanation there would be repeating. There
is no `Relates to:` — `python tools/adr.py --graph NNNN` derives the whole citation graph, which is
what 743 hand-maintained numbers across 95 files were approximating.

**A section number is a public identifier.** `0007 § 3` is cited from other ADRs, from `docs/spec/`,
from `docs/agent/loop-goal.toml` and from doc comments in `crates/`. Renumbering a section silently
breaks every one of them, so sections are appended (`§ 3a` beside `§ 3`) and never renumbered — and
`tools/adr.py` checks that every citation still names a section that exists.

**An ADR's body always states the current rule.** When a later decision changes an earlier one, the change
is folded into the earlier ADR's text, and the two carry one-line cross-links — never a paragraph in one
file describing what another file changed. So there is no patch to apply while reading, and no such thing
as a section that was true once. If you find a body that disagrees with a cross-link, the body is the bug.

**Measured numbers.** Each ADR quotes only its own measurements, and every one is guarded by a test in
[`benches/abi-probe`](../../benches/abi-probe/). The tests are authoritative; a number written anywhere
else is a copy that can go stale.

## Where to look

One row per topic, naming the **one** file that owns it. Read the row, open the file, stop. The left column
is the search surface — it carries the keywords and PHP spellings you are likely to arrive with; the right
column is only the destination. `python tools/brief.py --where <keyword>` prints just the rows that match,
so you never have to open this file to route a topic.

| Doing this | Open this |
|---|---|
| Deciding what to build next; checking what exists | [docs/implementation-plan.md](../implementation-plan.md) — the status block, then the table that routes to the milestones. The plan of record. |
| Scoping one milestone | `python tools/plan.py --show M8` — that milestone alone, out of [docs/plan/](../plan/). `--show M8:verify` is its acceptance paragraph by itself |
| What was decided before M0 — the architecture, the value representation, the unsafe policy, the verification strategy | [docs/plan/design.md](../plan/design.md) — the frozen half of the plan |
| How long a milestone takes; what a week of the original estimate is worth in loop-days, and what breaks that conversion | [docs/plan/velocity.md](../plan/velocity.md) — measured against `git log`, applied per class of work, gating nothing |
| Which file holds a thing; where a symbol is defined; what a module is for | `python tools/brief.py` — one line per module, plus the file:line of the definitions most often searched for |
| Something that looks like it should work and does not — a `Core` member's four required edits, a `.nvst` case that skips a leg, an Novis shape that will not compile | [docs/agent/playbook.md](../agent/playbook.md) — the trap list, append-mostly |
| The *shape* of something you are about to write — a commit message, a `.nvst` case, a `Core` member, an ADR, a diagnostic code, a splice patch | [docs/agent/conventions.md](../agent/conventions.md) — skeletons plus a worked example CI runs |
| Citing a document from code or from a doc — whether a link is relative or absolute from the root, why a moved `.rs` file changes no link, what `check-links.py` skips | [docs/agent/conventions.md](../agent/conventions.md) § *Citing a document* |
| Running the build, the tests, clippy and fmt; what "verified" means for a session; `splice.py`, `plan.py`, WSL, valgrind, and why a shell never writes a file here | [docs/agent/commands.md](../agent/commands.md) |
| How wide something may run on this machine — jobs, cores, `nproc`, the valgrind sweep's width, `NVS_JOBS`, why WSL's core count and not the host's, what is probed once and cached | `python tools/machine.py`, and [docs/agent/commands.md](../agent/commands.md) § *How wide anything runs* |
| Setting up a new development machine, or moving development to one — what to install, why Windows needs WSL, why PHP 8.5 goes on both sides at the same version, what a `git clone` does not carry, how to prove the machine is right, how to pick up where the last machine stopped | [docs/setup.md](../setup.md) |
| Cutting a release — the version bump, the changelog, the tag, the draft; what to set up on GitHub once; why nothing is written until every binary is green; why a release is never an agent's | [docs/release.md](../release.md), and `python tools/release.py --preview <bump>` to see one without running anything |
| Running Novis in a container — which tag and which variant, the `0.0.0.0` bind, health probes, why `docker stop` sends `SIGKILL`, verifying an image, why not Alpine | [docs/docker.md](../docker.md); [docker/Dockerfile](../../docker/Dockerfile) is the image itself |
| What a decision has already settled — one sentence per rule, with its ADR | [ground-rules.md](ground-rules.md), or `python tools/brief.py --where <keyword>` to route straight past it |
| Explaining a decision to a **person** — the plain-language summary the website publishes, what is in it, how to bring it level with the ADRs, why its prose carries no cross-references | `python tools/decisions.py`, and [docs/agent/decisions-summary.md](../agent/decisions-summary.md) for the pass. The summary itself is [docs/decisions.toml](../decisions.toml); `docs/decisions.md` and the site's copy are generated from it |
| Where Novis deliberately behaves differently from PHP — every divergence, with the ADR that owns it; what `nvs convert` can never tier **E**; why the `.phpt` pass rate is structurally lower | [divergences.md](divergences.md) |
| Writing a new ADR, amending an existing one, changing a status, or adding a `### N.` section — claiming the next free number, the filename, the back-link the amended ADR owes, the routing row, the ground-rules bullet, the index table | `python tools/adr.py --draft` then `--new FILE`; `--fold NNNN --into "…"`, `--set-status NNNN STATUS`, `--next-section NNNN` |
| Auditing the ADR set itself — a broken cross-link, a `§ N` citation into a section that no longer exists, an `Amends:` with no matching `Amended by:`, an ADR missing from an index, changelog prose in a body, what one ADR cites and is cited by, which files carry the most untrimmed rationale | `python tools/adr.py`, `--graph NNNN`, `--stats`, `--orphans` |
| Writing anything under `docs/` — where a fact lives, folding a changed decision, the length targets nothing enforces | [docs/agent/doc-style.md](../agent/doc-style.md) |
| What one loop session may read, and how a goal narrows it | `python tools/orient.py`, selected by `[context]` in [docs/agent/loop-goal.toml](../agent/loop-goal.toml) |
| Setting a new loop goal; how many slices a session should take; why the context ceiling is 200k; what to pre-authorize so a run never halts on `BLOCKED` | [docs/agent/loop-authoring.md](../agent/loop-authoring.md), and `python tools/loop-stats.py` for the numbers it rests on |
| Adding, reordering, renumbering or retiring a goal on the chain; what a goal's three files start as; why 21–49 are free | `python tools/chain.py` — [docs/agent/commands.md](../agent/commands.md) for the tool, [docs/agent/goals/README.md](../agent/goals/README.md) for the contract |
| Whether a feature is *finished* — what a member, a language feature or a directive owes before it counts: its tests from both sides, its three website examples, its measured cost, the program written to break it; where each goes; how the loop that writes them is generated | `python tools/dossier.py`, and [0134](../decisions/0134.md) for why the four |
| Exceptions, the call ABI, helper signatures, panic containment | [0002](../decisions/0002.md) — the only normative copy of the calling convention |
| Extensions, wasm, WIT, `.nvsx` | [0003](../decisions/0003.md) |
| Whether a stdlib feature belongs in `Core`, in the default binary, in an extension or nowhere; which PHP extension maps to what; whether a C dependency is acceptable | [0051](../decisions/0051.md) |
| `FFI`, `dl()`, native modules, stream wrappers, `php://`/`phar://`, `shmop`/`sysv*`/APCu, `eval`, `putenv`, `setlocale`, or "why can't userland do X at all" | [0052](../decisions/0052.md) |
| Whether a `.nvsx` can be an injection sink or source, `tainted`/`secret` at an extension call, what the manifest may declare | [0055](../decisions/0055.md) |
| What a `Core` member looks like — argument order, options, failure signalling, naming, mutation, callbacks; whether a PHP built-in survives at all | [0063](../decisions/0063.md) for the shape rules; [docs/spec/01-core-library.md](../spec/01-core-library.md) for every signature |
| How a `Core` member takes a fixed-key shape argument, a discriminated union of two shapes, why it must be written at the call site, and how it reaches the ABI | [0135](../decisions/0135.md) for the shape parameter; [0063](../decisions/0063.md) R2 for the trailing options bag it generalises |
| Where an implemented `Core` member's documentation lives — descriptions, parameter names, shape keys, errors; `nvs meta --json`; who wins when the spec and the registry both speak | [0117](../decisions/0117.md) |
| "What happened to `<php_function>`?" — any PHP built-in by name, and whether it became a member, a construct or nothing | [docs/spec/02-php-migration.md](../spec/02-php-migration.md), one row per name; `python tools/check-migration.py --report` lists what is still undecided |
| Durations and dates — `30s`/`1h30m` literals, `strtotime`, `DateTime` arithmetic, `sleep`, timeouts, why there is no `shift` | [0070](../decisions/0070.md) for the literal; [docs/spec/01-core-library.md](../spec/01-core-library.md) § 4 for `Core\Time` |
| Date *patterns* — `date()`/`strftime` letters, CLDR `yyyy-MM-dd`, which letters exist, quoting, why a month name is English and there is no locale | `crates/nvs-stdlib/src/cldr.rs`'s module docs — the one grammar `DateTime::format` and `Time::parse` share |
| Combining two arrays — `array_merge`, `array_replace`, `array_combine`, `$a + $b`, `array_merge_recursive`, why there is no `Arr::merge`, `preserveKeys`, `array_splice`, `array_pad`, `array_walk` | [0069](../decisions/0069.md) |
| Weighing memory against safety, speed or simplicity | [0004](../decisions/0004.md) |
| `nvs.toml` directives, `Core\Config::set`, limits, capabilities, changeability classes | [0005](../decisions/0005.md) |
| Development versus production, `APP_ENV`, `APP_DEBUG`, `DEBUG = True`, `NODE_ENV`, `RAILS_ENV`, staging, `Core\Env::mode`, why no environment variable is read, serving mixed applications from one host | [0091](../decisions/0091.md) |
| Logging and debug output — `Core\Log`, log levels, JSON Lines versus a readable line, `var_dump`/`print_r`/`var_export`, `Core\Debug::dump`, colour in a terminal, a collapsible dump in a browser, why there is no format argument, log forging | [0092](../decisions/0092.md) |
| The config file's *format* — why TOML, how a list/boolean/hash-pin is spelled, whether `nvs.toml` is a project manifest | [0064](../decisions/0064.md) |
| Splitting configuration across files — `[[include]]`, `conf.d`, an optional file that may not exist, `--config` more than once, where `nvs.toml` is found, what a relative path in it means, which file wins when two set the same key, config file permissions, a database or SMTP password out of `/run/secrets`, SOPS, `age`, sealed or encrypted secrets, `nvs config check`/`dump` | [0103](../decisions/0103.md) |
| Per-application limits, capabilities or mode; giving one site different rights from another on one host; what "an application" even is when there is no server; `[[app]]` | [0104](../decisions/0104.md) |
| Where a capability is actually checked; how a `Core` member declares it needs one; why a denied capability throws rather than escalating; what a check costs; what proves no member routes around it | [0118](../decisions/0118.md) |
| Changing a running server — `nvs ctl reload`, the control socket, adding or replacing an extension without a restart, which directives still need one, why there is no control port | [0078](../decisions/0078.md) |
| Running Novis as a Windows service or a systemd unit — `nvs service install`, NSSM/WinSW, `sc create`, `ImagePath` quoting, which service account, a hardened unit file, `Type=notify`, `ExecReload`, why a bundle cannot install itself | [0093](../decisions/0093.md) |
| The built-in HTTP server — what `nvs serve` is for and what it is not, mount points and several entry points under one root, serving static files, why there is no TLS listener, h2c, FastCGI or compression, what a proxy in front is trusted to assert, `X-Forwarded-For` and the client IP, connection timeouts, the in-flight ceiling, a health endpoint | [0097](../decisions/0097.md) |
| File uploads — `$_FILES`, `tmp_name`, `move_uploaded_file`, `upload_max_filesize`, streaming a large upload, how big an upload may be, where an uploaded file is stored, writing a stream to disk, `Core\IO::writeStream` | [0105](../decisions/0105.md) |
| Temporary files and directories — `tmpfile`, `tempnam`, `sys_get_temp_dir`, `Core\IO::temporaryDir`, who deletes a temp dir and when, the orphan sweep, `nvs tmp clean`, `[io] temp_root`, `[debug] keep_temporary` | [0131](../decisions/0131.md) |
| `spawn script`, isolates, the request boundary | [0006](../decisions/0006.md) |
| Caching between requests, APCu, `Core\Cache`, why a cached value is copied | [0059](../decisions/0059.md) |
| `[cache.shared]`, the `cache.shared` grant, reaching a store over a Unix socket, `unix:` in a store URL, a socket path in `[db.<name>] host`, why a program may not name one | [0142](../decisions/0142.md) |
| `Core\Session`, `session_*`, where a session record lives, `[session] backend`, session locking, session garbage collection, what a presented session id is checked against | [0139](../decisions/0139.md) |
| `include`/`require`, loading another file into the current frame | [0021](../decisions/0021.md) |
| Case sensitivity, whether `IF`/`TRUE` parse, whether `new httpclient()` resolves, or why a `require` that works on Windows must work on Linux | [0062](../decisions/0062.md) |
| Autoloading, `spl_autoload_register`, PSR-4, Composer's `vendor/autoload.php`, or enumerating classes nothing references by name | [0061](../decisions/0061.md) |
| Uninitialized properties, `undefined`, why a typed property can't silently be `null`/zero | [0022](../decisions/0022.md) |
| `lateinit`, deferring a property's first assignment past the constructor, DI/setter injection | [0038](../decisions/0038.md) |
| `clone`, `serialize`/`unserialize`, `__clone`/`__sleep`/`__wakeup`, or how a value crosses a `spawn` boundary | [0023](../decisions/0023.md) |
| Live cache invalidation, picking up an edited `.nvs` file without a restart | [0017](../decisions/0017.md) |
| The on-disk compiled-artifact cache — file layout, header format, why a tampered file is a cache miss, eviction | [0042](../decisions/0042.md) |
| A portable single-file executable, `nvs build --compile`, bundling a CLI app's source | [0048](../decisions/0048.md) |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys, arithmetic result types, user-defined generics | [0007](../decisions/0007.md) |
| Whether a `for` header may declare its own counter, and what an init clause holding both a declaration and an expression is diagnosed as | [0109](../decisions/0109.md) |
| `decimal`, money, `bcmath`, `gmp`, big integers, why floats aren't used for currency | [0054](../decisions/0054.md) |
| `string` vs `bytes`, the UTF-8 guarantee, text/binary conversion, what `length` counts | [0009](../decisions/0009.md) |
| `foreach` over an object, `Iterator`/`ArrayAccess`/`Countable`, generators, `yield`, lazy streaming | [0053](../decisions/0053.md) |
| `var`, local type inference, why `$x = "foo";` doesn't need its type spelled out | [0037](../decisions/0037.md) |
| PHP's `(int)$x` legacy cast syntax, why it doesn't parse | [0034](../decisions/0034.md) |
| A conversion that shouldn't throw, `as ?int`, `tryParse`, validating untrusted input without `try`/`catch` | [0066](../decisions/0066.md) |
| A one-line `try`/`catch`, the expression form `expr catch (Class $e) => value`, a fallback value for a call that throws, why an arm may `throw` but not `return`, `catch (Throwable) => …` warning, Swift's `try?`, Zig's `catch`, Go's comma-ok | [0119](../decisions/0119.md) |
| A class picked at run time, `new $cls(...)`, `$cls::make()`, `$x instanceof $cls`, `Foo::class` as a value, a factory over a discriminator string, `class<T>`, why a bare `string` is `E0496` | [0125](../decisions/0125.md) |
| A field named at run time, `$obj->$name`, a sort column or patch key from a request, mass assignment, `property<T>`, why a computed member name is `E0235` and where that refusal now lives | [0126](../decisions/0126.md) |
| Images — `gd`, `exif`, `imagick`, `imagecreatefromjpeg`, `imagecopyresampled`, `getimagesize`, `Novis\Image`, `nvs/image`, resizing, cropping, thumbnails, `srcset` variants, WebP/AVIF/JPEG XL, a pixel bomb and `[image] max_pixels`, EXIF orientation, ICC profiles, stripping GPS, comparing two images in a test, perceptual hashes, BlurHash, text on an image, QR codes, rasterising an SVG | [0120](../decisions/0120.md) |
| PDF — HTML to PDF, `dompdf`, `mpdf`, `tcpdf`, `fpdf`, `wkhtmltopdf`, headless Chromium, invoices and reports, `Novis\Pdf`, `nvs/pdf`, the no-I/O render and its asset map, the CSS subset and the dropped-declarations report, an inert byte-reproducible output, the browser escape hatch | [0121](../decisions/0121.md) |
| Rasterising a PDF — Ghostscript, ImageMagick's PDF delegate, `pdftoppm`, Poppler, pdfium, a thumbnail of an uploaded PDF, counting a PDF's pages, `Image::open` on a PDF, `page` and `dpi`, an encrypted PDF, a PDF bomb, `hayro` | [0128](../decisions/0128.md) |
| Spreadsheets — xlsx, xlsm, xlsb, xls, ods, PhpSpreadsheet, uploaded workbook imports, export and report generation, `Novis\Spreadsheet`, `nvs/spreadsheet`, formulas as typed values, formula injection, the macro refusal, template fill, explicit evaluation | [0123](../decisions/0123.md) |
| HTML parsing — `DOMDocument::loadHTML`, PHP 8.4's `Dom\HTMLDocument`, tag soup, scraping, sanitizing rich text, mXSS, `html5ever`, the tree shared with `Core\Xml`, why there is no HTML mode on the XML parser | [0122](../decisions/0122.md) |
| PHP 8.6 — partial function application `f(?, $x)`, `return` in `finally`, `return $v` in a constructor, `let`/`is` as identifiers, defaults on `readonly` properties, `clamp`, `Time\Duration`, `Io\Poll`, `session.use_strict_mode`, everything 8.6 deprecates | [0124](../decisions/0124.md) |
| Whether an `if`/`while`/`?:`/`&&`/`!` condition needs an explicit `as bool`, PHP truthiness | [0035](../decisions/0035.md) |
| PHP's `and`/`or`/`xor` keyword operators, why they don't parse | [0045](../decisions/0045.md) |
| `<?php` as an open tag, PHP's `die` keyword | [0049](../decisions/0049.md) |
| `list($a, $b) = $pair;`, PHP's `list()` destructuring spelling | [0050](../decisions/0050.md) |
| Restricting a parameter to a fixed set of values (`#[ExpectedValues]`), `"a"\|"b"` literal types, a subset of an enum's cases | [0047](../decisions/0047.md) |
| `#[Attribute]`-style metadata, annotations, `Core\Attributes`, why there's no attribute base class | [0046](../decisions/0046.md) |
| Hydrating a class from JSON or a database row — `#[Json\Derive]`, `#[Db\Derive]`, `JsonSerializable`, `PDO::FETCH_CLASS`, serde-style derives, reporting every bad field of a submitted form | [0071](../decisions/0071.md) |
| Routing — `#[Route]`, URL patterns and `{id}` placeholders, reverse URL generation, why the router does not dispatch, `Core\Router` | [0077](../decisions/0077.md) |
| Whether one method may serve several verbs under one route name, why there is no wildcard verb, and why a path is never derived from the declaring class | [0110](../decisions/0110.md) |
| Reading the current route and its parameters, a `405` and its `Allow:` header, binding a query parameter with `#[Query]`, an optional `{page?}` segment, narrowing a capture to a closed set, why the router refuses a regex, absolute links and `[app]`/mount `origin`, `Core\Request::mount()` for subdomain multi-tenancy, who enforces `#[Access]` | [0102](../decisions/0102.md) |
| Writing a CLI program — colour and `Cli\Text`, why `echo` neutralizes escape sequences, prompts and `select`, password input, progress bars and in-place output, `#[Command]`/`#[Option]` argument parsing, `--help` and shell completions, `isTty`, terminal width | [0086](../decisions/0086.md) |
| Python — what Novis claims against it and what may never be said, whether a script needs `<?nvs`, `#!`/shebang, why there is no REPL or interactive shell, `pip`/virtualenv/PyInstaller, and how the two are benchmarked | [0100](../decisions/0100.md) |
| Trojan Source, right-to-left overrides, `U+202E`, a comment that renders as code, homoglyphs, zero-width characters, or why a non-ASCII identifier does not compile | [0087](../decisions/0087.md) |
| Porting a PHP codebase — `nvs convert`, its two modes, what a `TODO(convert:…)` means, why the default output does not run, how a rewrite is proven, where the rule table lives, which PHP parser and which PHP versions | [0089](../decisions/0089.md) |
| Running several things at once — `Task::all`/`::map`, `parallel_map`, a task deadline, cancellation, work after the response is sent, `fastcgi_finish_request`, why there is no job queue | [0072](../decisions/0072.md) |
| End-of-script work — `register_shutdown_function`, a shutdown hook, the exit reason, what still runs after `exit` or an uncaught throw, `Core\Script::onExit` | [0127](../decisions/0127.md) |
| Cron, scheduled jobs, a nightly task, running something once across a fleet, `[[schedule]]` | [0073](../decisions/0073.md) |
| Response security headers, CORS, cookie defaults, HSTS, CSP; and outbound timeouts, retries, backoff, idempotency keys | [0074](../decisions/0074.md) |
| How a header is parsed, folded headers, a header with no colon, request smuggling, CRLF, a cookie's *name* and its `__Host-` prefix, multipart part counts, reserved Windows filenames, a path component that resolves elsewhere | [0095](../decisions/0095.md) |
| Whether a route may omit an authorization check, `#[Access]`, roles and policies on a handler, CSRF enforcement defaults | [0096](../decisions/0096.md) |
| Rate limiting, throttling logins, per-tenant quotas, `Retry-After`, `429`, load shedding | [0075](../decisions/0075.md) |
| Metrics, Prometheus, OpenTelemetry, distributed tracing, `traceparent`, `Core\Metrics`, label cardinality | [0076](../decisions/0076.md) |
| Measuring adoption — usage telemetry, analytics, phoning home, opt-in counters, `nvs telemetry`, `nvs update-check`, the version check, the upload endpoint, why there is no `X-Powered-By`/`expose_php` header | [0130](../decisions/0130.md) |
| Regex, `preg_*`, `Core\Regex`, ReDoS, backreferences, lookaround | [0056](../decisions/0056.md) |
| Why a literal regex/URI/format string is checked by `nvs check`, compile-time preparation | [0057](../decisions/0057.md) |
| `enum`, enum cases, backing type, anything enum-shaped | [0010](../decisions/0010.md) |
| `static`, `global`, scoping, where state may live at all | [0008](../decisions/0008.md) |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [0011](../decisions/0011.md) |
| `callable`, first-class callable syntax (`Foo::bar(...)`), `__invoke`, calling an object with `()` | [0027](../decisions/0027.md) |
| A typed callback — `callable(int): string`, a callable type's arity and variance, why a `fn` literal needs no parameter annotations, what retired `CoreTy::CallableTo` | [0136](../decisions/0136.md) |
| A doc comment, `///`, a docblock, `@param`/`@return`/`@throws`, why there is no PHPDoc, `@see`, `@example`, `nvs doc`, `--strict-docs` | [0137](../decisions/0137.md); [0117](../decisions/0117.md) for a `Core` member, which carries none |
| The pipeline operator, `\|>`, the hole `$_`, method chaining, a fluent interface on a `string`/`array<T>`, why PHP 8.5's `\|>` spelling does not work here, `#[Fluentable]` | [0098](../decisions/0098.md) for the operator; [0063](../decisions/0063.md) R17-R19 for why there are no methods on scalars |
| Anonymous functions, `fn`, arrow functions, closure capture, `use (...)`, recursive closures | [0031](../decisions/0031.md) |
| By-reference parameters, `inout`, `foreach (… as inout $v)`, the retired `&$x` spelling, whether a call site marks an argument it writes | [0107](../decisions/0107.md) |
| `__toString`/`Stringable`, `__destruct`, `__isset`/`__unset`, `unset()` on an object property, `__debugInfo`, `__set_state`, or "what happened to PHP magic method X" | [0028](../decisions/0028.md) |
| `stdClass`, an anonymous object literal `{a: 1}`, the `object` type, an inline `{name: T}` shape type | [0036](../decisions/0036.md) |
| Naming conventions, `PascalCase`/`camelCase`/`SCREAMING_SNAKE_CASE`, acronym spelling, identifier casing | [0029](../decisions/0029.md) |
| Leading underscores in identifiers, whether the constructor is `__construct` or `constructor` | [0030](../decisions/0030.md) |
| Whether a member may omit `public`/`protected`/`private`, what an omission means, `var $x`, a bare `private(set)`, whether a plain constructor parameter needs one | [0094](../decisions/0094.md) |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$argv`, or anything PHP populates ambiently | [0012](../decisions/0012.md) |
| Comparing two objects with `<`/`>`/`<=>`, operator overloading, `Comparable` | [0013](../decisions/0013.md) |
| `==` vs `===`, loose comparison, type juggling, why `"1" == 1` does not compile, what two strings/arrays/objects compare by, `__equals`, comparing a `mixed` | [0090](../decisions/0090.md) |
| What "the same value" means — strict identity, `in_array`'s strict flag, `array_search`, `array_unique`, whether two objects/arrays/`NaN`/`-0.0` match | [crates/nvs-runtime/src/identity.rs](../../crates/nvs-runtime/src/identity.rs) — one row per representation, and the hash that agrees with it |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [0014](../decisions/0014.md) |
| `class_alias`, `use … as …`, or a `type` alias | [0015](../decisions/0015.md) |
| Whether a name needs a leading `\`, what a qualified name resolves against, reaching a root-level name from inside a namespace | [0113](../decisions/0113.md) |
| `trait`, horizontal code reuse, mixins, `insteadof`, how a PHP trait migrates | [0043](../decisions/0043.md) |
| Code coverage, call tracing, the per-call profiler, `Core\Debug`, the `[debug]` config section | [0018](../decisions/0018.md) |
| A timeline/flame-chart view combining calls with GC pauses and isolate boundaries, exporting to speedscope | [0041](../decisions/0041.md) |
| `Core\Reflect`, `ReflectionClass`-equivalents, `Core\Ast`, runtime introspection or source parsing | [0019](../decisions/0019.md) |
| Uncaught exceptions, memory/CPU-limit fatals, internal panics, `Core\Fatal`, `Core\Log` | [0020](../decisions/0020.md) |
| Whether one request can take the server down; a worker that panics outside a helper, aborts, or is alive but stuck; `abort()`, `SIGSEGV` from the engine's own recursion, `SIGBUS`, W^X; a decoder's depth limit; a helper that never yields a core; blocking syscalls and the blocking pool; what `max_in_flight` really admits; why there are no worker processes; a request the client abandoned — the closed tab, a hung request holding a worker, `ignore_user_abort`, `connection_aborted` | [0106](../decisions/0106.md) |
| epoll/kqueue/IOCP, the reactor, what wakes a parked task, why a socket read looks blocking and is not, `WouldBlock`, how big a coroutine's stack is and who pays for it | [0115](../decisions/0115.md) |
| A `Future` in a runtime that is not `async`, `block_on`, what a `Waker` may do, `hyper`'s connection future, waking from another thread, why none of this is an executor | [0138](../decisions/0138.md) |
| What an isolate's "arena" actually is, whether entering one maps memory, what a wholesale release runs, what one isolate costs, why a crossing may move rather than copy, where a byte cap attaches | [0116](../decisions/0116.md) |
| XSS, SQL injection, command/header/path injection, taint tracking, `tainted string`, `Core\Html\Markup` | [0024](../decisions/0024.md) |
| Double escaping, `&amp;amp;`, what an escaper *returns*, why `Core\Html::escape` answers `Markup` and `Core\Db::quoteIdentifier` a `string`, whether `Markup` converts back, `Core\Html::toSource`, `Core\Html::sanitize`'s return type | [0133](../decisions/0133.md) |
| Whether a given parameter is a sink, what an unclassified one does, what `echo` writes to in a request / a CLI / an isolate / a scheduled run, how a JSON or plain-text response body is written, `Core\Response::json` | [0088](../decisions/0088.md) |
| A database — `Core\Db`, `PDO`/`mysqli`/`pgsql`/`sqlite3`, drivers, connections, prepared statements, transactions, result rows, an ORM | [0067](../decisions/0067.md) — signatures in [spec § 18](../spec/01-core-library.md) |
| Writing a database driver — `crates/nvs-db`, which wire crate backs which driver, `postgres-protocol`/`mysql_common`/TDS, why there is no `sqlx` and no `tokio`, TLS on a database socket, whether a connection is busy, why the drivers are an enum and not a trait | [0132](../decisions/0132.md) |
| SSRF, fetching a user-supplied URL, `Core\Http\Client`, the `net.connect` address policy, DNS rebinding | [0058](../decisions/0058.md) |
| `exec`/`system`/`shell_exec`/backticks/`proc_open`, running another program, shell injection | [0044](../decisions/0044.md) |
| Passwords, API keys, credentials, `secret string`, why a value can't be echoed/logged/dumped | [0033](../decisions/0033.md) |
| Migrating a PHP user table — `password_hash`, `PASSWORD_DEFAULT`, a stored `$2y$` bcrypt hash, `password_needs_rehash`, the login-time upgrade to Argon2id, the bcrypt cost ceiling | [0129](../decisions/0129.md) |
| JWT, CSRF tokens, TOTP, signed cookies, or whether OAuth/WebAuthn/SAML belong in `Core` | [0060](../decisions/0060.md) |
| Running Novis client-side in a browser, a wasm32 compile target, `Core\Browser` — retired, and what would reopen it | [0025](../decisions/0025.md) |
| The PhpStorm plugin, `nvs-lsp`/`nvs fmt` client wiring, what "IDE integration" covers | [0016](../decisions/0016.md) |
| `nvs fmt`'s style — indentation, braces, quoting, trailing commas, why it never reflows | [0039](../decisions/0039.md) |
| What `nvs fmt` refuses to rewrite — renames, missing keywords, member order — and how an editor composes it with quick fixes on save | [0039](../decisions/0039.md) §§ 9-11, [0040](../decisions/0040.md) § 3 |
| The VS Code extension's feature catalog, why a minimal `nvs-lsp` ships in M4B, `nvs-syntax`'s resilient parse mode | [0040](../decisions/0040.md) |
| What the resilient tree *is* (trivia + an offset index, not a second CST), what `nvs-lsp` is built on and why not `tower-lsp`, the M4B request set, syntax highlighting's two layers and what each must colour, the `.lspt` case format | [0099](../decisions/0099.md) |
| Hiding a credential on a shared screen, blurring a `secret` literal in the editor, `nvs/redactions`, marking a `tainted` value with a glyph, and what a decoration still leaks (Search, diffs, the clipboard) | [0101](../decisions/0101.md) |
| Find-all-references, occurrence highlight, CodeLens, type hierarchy, dimming an unused member; whether the editor completes a route name, a config directive or a framework's conventions; Emmet and HTML/CSS/JS editing inside a template region; generating a missing member or override; what `nvs dap` must report for the debugger UI to be deep; `nvs check --json` | [0108](../decisions/0108.md) |
| Typing a PHP built-in's name and getting the Novis one; where the candidate list and the answer each come from; why a completion item may insert nothing; `nvs.completion.phpNames` | [0111](../decisions/0111.md) |
| Narrowing an `array<mixed>` annotation to the type of the literal under it; what "narrowest" means once literal types widen; where that synthesis lives and why no compile path calls it; why the action never runs on save | [0114](../decisions/0114.md) |
| Writing a test case — the `.nvst` sections, `--EXPECTF--`'s escapes, `--ORACLE--`/`--ORACLE-DIVERGES--`, how `nvs test` decides pass or fail, importing a `.phpt` | [crates/nvs-test/src/lib.rs](../../crates/nvs-test/src/lib.rs)'s module doc — the one home for the format |
| Testing a program *written in* Novis — `#[Test]`, `Core\Test`, assertions, doubles, fixtures, parameterized cases, property testing, snapshots, `#[Bench]`, mutation testing, why PHPUnit's mechanism does not port | [0079](../decisions/0079.md) — distinct from the `.nvst` row above, which is Novis's own conformance suite |
| PHPUnit, Pest, Mockery, Infection, PHPBench, PHPStan, Psalm, PHP CS Fixer, PHP_CodeSniffer, Rector, Xdebug, Deptrac, phpDocumentor, Faker — any PHP dev tool by name, and where its job landed | [tooling-parity.md](tooling-parity.md) — an index like `divergences.md`; the ADR each row names is the rule |
| Third-party licenses, attribution, what `nvs info` prints, whether a new dependency's license may ship | [0065](../decisions/0065.md) |
| Updating a crate, the Rust toolchain, a CI action or the PHP oracle; SemVer, what a break costs a release, deprecation, MSRV, pinning, a stale dependency | [0068](../decisions/0068.md) for the policy; [docs/agent/dependency-update.md](../agent/dependency-update.md) for the procedure — a pass the user fires by hand |
| Cross-machine performance history, callgrind instruction counts, why CI guards use wall-clock ratios | [0026](../decisions/0026.md) |
| Why Novis loses a `tools/bench.py` case to PHP — a slow string, a slow array, an allocation, a name lookup — and which piece of work closes it | [docs/perf/userland-gap.md](../perf/userland-gap.md) — a ledger that shrinks as items land; [benches/userland/README.md](../../benches/userland/README.md) owns what a case *is* |
| Concrete *spelling* an ADR left open — file modes, the declaration-slot grammar, `as`, the `bytes` literal | [docs/spec/00-overview.md](../spec/00-overview.md) — the ADR owns semantics, this owns syntax |
| The one-file reference for language *users* — `docs/novis.md`, what a search engine or a language model reads, how it is generated and proven, the chapter under `docs/reference/` that owns a topic | [docs/reference/README.md](../reference/README.md) — the chapters are prose derived from the ADRs and the registry, never a second home for a rule; `python tools/reference.py` regenerates, `python tools/proof.py` judges |
| How a control-flow statement lowers — `if`/`while`/`for`/`foreach`/`switch`/`match`, `break`/`continue`, where a `finally` runs | [crates/nvs-ir/src/lower/control.rs](../../crates/nvs-ir/src/lower/control.rs)'s methods, each with its own doc comment; `continue` inside a `switch` is § *Decisions taken at project start* below |
| A decision with no ADR — thread-per-core, value layout, safepoints, shared-nothing requests, SIMD | § *Decisions taken at project start* below for **why**; the plan's § *Architecture* for the **mechanics** |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](../../benches/abi-probe/) — authoritative; docs quote them and can lag |
| Who Novis is for, what it claims about itself, whether "PHP compatible" may be written anywhere, why this does not end where Hack ended, or which of two slices to build first | [0080](../decisions/0080.md) |
| Explaining `tainted`/`secret` to a non-technical reader, an auditor or a developer; what PHP, JS, Python, Java, C#, Go, Ruby and Rust have instead; why "XSS is impossible" may not be written | [docs/why-tainted-and-secret.md](../why-tainted-and-secret.md) — the audience-facing argument only; every rule it cites is owned by the ADR it links |
| Third-party libraries — the registry, a git dependency, `package.toml`/`package.lock`, `nvs add`/`fetch`/`update`/`vendor`/`audit`/`publish`, version resolution, dependency hell, supply-chain attacks, install scripts | [0081](../decisions/0081.md) |
| What authority a dependency, a hand-vendored directory or an overriding file holds; `[grants]` and how one is looked up; a capability a package declares *optional*, and `Core\Cap::has`; whether top-level code is exempt; the roster of capability names | [0112](../decisions/0112.md) |
| The framework — `nvs new`, controllers, middleware, auth flows, mail, i18n, storage, pagination, `Web\*`, where the ORM is, where the service container is, why there is no template engine | [0082](../decisions/0082.md) |
| WebSocket, SSE, a long-lived connection, push, broadcast, `Core\Topic`, presence, or what happens to an isolate when a socket outlives its request | [0083](../decisions/0083.md) |
| Background jobs, `Core\Queue`, retries, dead-letter, workers, `nvs work`, transactional enqueue, or why there is no Redis backend | [0084](../decisions/0084.md) |
| OpenAPI, Swagger, an API contract, `#[Api]`, generated docs, or detecting a breaking API change | [0085](../decisions/0085.md) |
| Connection pooling, a reused database connection, what a connection reset must remove, `cores × max` sizing | [0067](../decisions/0067.md) § 13 |
| What the language should *do* beyond the two spec files | unwritten. `docs/spec/` holds `00-overview.md` and `01-core-library.md` and nothing else — say so rather than inferring semantics. |
| why CI skipped the Windows leg, why a check says *Skipped*, what a push runs versus a release, the nightly run, adding a job to ci.yml, `tools/ci-changes.py` | [ci.yml](../../.github/workflows/ci.yml), with the lane table in [tools/ci-changes.py](../../tools/ci-changes.py) |
| `::class`, `static::class`, `$obj::class`, `get_called_class`, `get_class` | this ADR |
| Schema migration, `CREATE TABLE`, `ALTER TABLE`, `doctrine/migrations`, Laravel `Schema::create`, Rails `db:migrate`, `information_schema`, dumping an existing database, `nvs schema plan`/`apply`/`dump` | [0145](../decisions/0145.md) |
| signing a URL, verifying a URL signature, a signed or expiring link, a signed AJAX endpoint, `Core\Signature`, `$uri->sign`, `Core\Router::urlSigned`, Laravel's `signedRoute` and `hasValidSignature`, an S3-style presigned URL | this file |
| an option that clears rather than sets; removing a component from a `Uri`; setting or removing one query parameter; `withQueryParameter`, `setQueryParam`, `http_build_query` over an existing URL; telling an omitted key from a written `null`; `{fragment: null}`; `array_key_exists` vs `isset` on an options array; why `""` does not clear | [0147](../decisions/0147.md) for the rule and the `Core\Uri` members that spend it; [0063](../decisions/0063.md) R2 for the bag it applies to |

**Adding a decision.** `python tools/adr.py --draft > .agent-tmp/adr.md`, fill in the prose, then
`python tools/adr.py --new .agent-tmp/adr.md`. That does steps 1, 2, 4, 5 and 7 below and half of 3 —
the number, the filename, the date, the back-link into every ADR you amend, the routing row, the
ground-rules bullet, the divergence row, the index table, and the audit — as one transaction that
restores every byte if the tree gained a finding. **Step 3's body edit and step 6 are yours**, and the
tool names the files that still owe them. The list is kept because it is what the tool is doing, and
because a reader who wants to know why an ADR touches five files has one place to find out.

1. Write the ADR file, following the shape every other one has: the metadata block, **In short**, then
   `## Context` / `## Decision` / `## Consequences` / `## Alternatives rejected` / `## Revisiting` /
   `## Verification` as needed — **in that order**, and using only those headings. The field set and
   the section order are both closed and both checked by `python tools/adr.py`.
2. Regenerate the index below with `python tools/adr.py --index` and paste it in. **The Decision cell
   is your ADR's own title**, so it cannot drift and there is nothing to write twice; before this was
   generated, 26 of the 102 cells had grown past 200 bytes and one had reached 836.
3. **If it changes a prior ADR's rule, edit that ADR's body to state the new rule** — in the same commit,
   not as a note about what changed. Then add a one-line `Amends:` to yours naming the ADR and section, and
   add your number to that ADR's bare `Amended by:` list. Nothing else. If folding leaves the earlier ADR
   with no unique content at all, delete it instead and update every reference; the number stays retired.
4. If the topic is one an agent will search for by keyword, add one row to the *Where to look* table above —
   the only one there is, since [AGENTS.md](../../AGENTS.md) points here rather than keeping a copy — and,
   only if it is a hard invariant, one **sentence** to [ground-rules.md](ground-rules.md).
5. If the decision makes a ported PHP program behave differently, add one row to
   [divergences.md](divergences.md). That register is the one home for the count, so your ADR states its
   own divergence and never a running total.
6. If it changes what a milestone builds, update that milestone's paragraph in
   [the plan](../implementation-plan.md) with a link and a headline, not a restatement.
7. Run `python tools/adr.py`. It refuses an unknown metadata field, a non-canonical or out-of-order
   heading, a broken link, a `§ N` citation into a section that does not exist, an `Amends:` with no
   matching `Amended by:`, an ADR missing from either index, a stale index table, and changelog prose
   in a body. All eight were real defects in this set before it existed.

`tools/brief.py` needs no update: it slices this file's tables and the plan's status block live. Two things
to get right in a new row, both for the reader rather than for a checker — nothing enforces either: keep the
**Decision** cell to one sentence, and write any `|` inside a cell as `\|`.

| # | Decision | Status |
|---|---|---|
| [0002](../decisions/0002.md) | Exceptions propagate by checked return, not by unwinding | Accepted |
| [0003](../decisions/0003.md) | Extensions are sandboxed WebAssembly components, not native shared libraries | Accepted |
| [0004](../decisions/0004.md) | Memory is spent for security, speed and simplicity | Accepted |
| [0005](../decisions/0005.md) | `nvs.toml` states defaults, not ceilings | Accepted |
| [0006](../decisions/0006.md) | Running another script is an in-process isolate, not a subprocess | Accepted |
| [0007](../decisions/0007.md) | Types are declared, checked, and never change by themselves | Accepted |
| [0008](../decisions/0008.md) | `static` marks a class member; there are no function statics and no `global` | Accepted |
| [0009](../decisions/0009.md) | `string` is text; binary data is a distinct `bytes` type | Accepted |
| [0010](../decisions/0010.md) | Enums are a closed, named integer type, not PHP's class-like construct | Accepted |
| [0011](../decisions/0011.md) | Functions and constants are class members; `Core` is the reserved namespace for built-ins | Accepted |
| [0012](../decisions/0012.md) | There are no superglobals; request, session, environment and CLI state are `Core` accessor classes | Accepted |
| [0013](../decisions/0013.md) | Ordering two objects requires `Comparable`; PHP's property-walk fallback is rejected | Accepted |
| [0014](../decisions/0014.md) | Property hooks feed a declared `PropertyObserver`; no undefined-property fallback, no `__call`/`__callStatic` | Accepted |
| [0015](../decisions/0015.md) | No PHP-style name aliasing; `type` aliases are the disciplined exception | Accepted |
| [0016](../decisions/0016.md) | IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native | Accepted |
| [0017](../decisions/0017.md) | The compiled-unit cache revalidates lazily and swaps one pointer, never a watcher or a restart | Accepted |
| [0018](../decisions/0018.md) | Coverage, tracing and profiling are safepoint-shaped probes, not a second compiled tier | Accepted |
| [0019](../decisions/0019.md) | Reflection and AST/source parsing are first-class `Core` features, not aftermarket extensions | Accepted |
| [0020](../decisions/0020.md) | Fatal errors reach user code through a reserved-budget ladder, never through `catch` | Accepted |
| [0021](../decisions/0021.md) | `require` is the only same-frame file-inclusion construct | Accepted |
| [0022](../decisions/0022.md) | Properties are definitely initialized at compile time; no observable uninitialized state | Accepted |
| [0023](../decisions/0023.md) | Two copy depths, neither customizable: `clone`, `serialize`, and the isolate boundary | Accepted |
| [0024](../decisions/0024.md) | Untrusted input is a distinct type; injection sinks demand laundering | Accepted |
| [0025](../decisions/0025.md) | The browser is a second compile target, not a second language | Retired |
| [0026](../decisions/0026.md) | Performance history is tracked by callgrind instruction counts; wall-clock stays for CI regression guards | Accepted |
| [0027](../decisions/0027.md) | `callable` is satisfied only by a closure; Novis has no `__invoke` | Accepted |
| [0028](../decisions/0028.md) | Closing the remaining magic methods: `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property | Accepted |
| [0029](../decisions/0029.md) | Identifier casing is a hard compiler error: `PascalCase` types, `camelCase` members, `SCREAMING_SNAKE_CASE` constants | Accepted |
| [0030](../decisions/0030.md) | No leading underscores anywhere; the constructor is spelled `constructor`, not `__construct` | Accepted |
| [0031](../decisions/0031.md) | `callable` is the only closure type; `fn` is the only closure literal | Accepted |
| [0033](../decisions/0033.md) | `secret`: a second compile-time qualifier for confidential values, composable with `tainted` | Accepted |
| [0034](../decisions/0034.md) | PHP's legacy `(T)expr` cast syntax is rejected; `as` is the only conversion spelling | Accepted |
| [0035](../decisions/0035.md) | A condition is judged by PHP's full truthy table; every other `bool` position stays checked | Accepted |
| [0036](../decisions/0036.md) | `object` is the opaque top of every class type; `{...}` builds an anonymous, methodless instance; an inline `{name: T, ...}` shape is Novis's one structurally-checked type | Accepted |
| [0037](../decisions/0037.md) | `var` infers a local's type from its initializer | Accepted |
| [0038](../decisions/0038.md) | `lateinit` defers a non-nullable object property's first assignment past the constructor | Accepted |
| [0039](../decisions/0039.md) | `nvs fmt` is the one canonical, unconfigurable formatting style, run on demand only | Accepted |
| [0040](../decisions/0040.md) | The VS Code extension is a deep, first-class client; `nvs-syntax` gains a resilient parse mode; a minimal `nvs-lsp` moves ahead of M10 | Accepted |
| [0041](../decisions/0041.md) | Trace/profile output gains a speedscope-evented timeline export, plus GC-pause and isolate-spawn trace events | Accepted |
| [0042](../decisions/0042.md) | The on-disk artifact cache is one immutable, self-describing file per compiled unit, verified before it is ever mapped executable | Accepted |
| [0043](../decisions/0043.md) | There is no `trait`; interface default/private methods plus explicit `by` delegation replace it | Accepted |
| [0044](../decisions/0044.md) | `Core\Process` is the one argv-only way to run another program; no shell, ever | Accepted |
| [0045](../decisions/0045.md) | PHP's `and`/`or`/`xor` keyword operators are rejected; `&&`/`\|\|` are the only logical connectives | Accepted |
| [0046](../decisions/0046.md) | Attributes are shape-literal metadata on declarations, retrieved structurally via `Core\Attributes` | Accepted |
| [0047](../decisions/0047.md) | A scalar literal or a named enum case is itself a type; unioning them declares a closed set | Accepted |
| [0048](../decisions/0048.md) | A portable single-file executable appends source to the host binary; rebundling is a build-time CLI step, not a runtime one | Accepted |
| [0049](../decisions/0049.md) | `<?php` and `die` are rejected; `<?nvs` and `exit` are the only spellings kept | Accepted |
| [0050](../decisions/0050.md) | `list(...)` is rejected; `[...]` is the only destructuring spelling | Accepted |
| [0051](../decisions/0051.md) | The standard library's tiers: what is `Core`, what ships native, what is an extension | Accepted |
| [0052](../decisions/0052.md) | Four closed doors: no FFI, no stream wrappers, no cross-request state, no `eval` | Accepted |
| [0053](../decisions/0053.md) | `Iterable`/`Iterator` are the only iteration interfaces; generators lower to state machines | Accepted |
| [0054](../decisions/0054.md) | `decimal` is a scalar type; `bcmath` and `gmp` are retired | Accepted |
| [0055](../decisions/0055.md) | Extension manifests carry `tainted` and `secret` qualifiers; an extension can only tighten | Accepted |
| [0056](../decisions/0056.md) | Regex runs on a linear-time engine by default; backtracking is opt-in and budgeted | Accepted |
| [0057](../decisions/0057.md) | Literal arguments to intrinsic `Core` calls are validated and prepared at compile time | Accepted |
| [0058](../decisions/0058.md) | Outbound connections carry an address policy; a tainted URL must be laundered and pinned | Accepted |
| [0059](../decisions/0059.md) | Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, and capped | Accepted |
| [0060](../decisions/0060.md) | A closed roster of application-layer security protocols lives in `Core` | Accepted |
| [0061](../decisions/0061.md) | `autoload` maps names to files at compile time; `Core\Program` enumerates what nothing names | Accepted |
| [0062](../decisions/0062.md) | Case sensitivity is a compiler property, never an OS property | Accepted |
| [0063](../decisions/0063.md) | `Core` API conventions: one shape for every built-in | Accepted |
| [0064](../decisions/0064.md) | Configuration is TOML, in `nvs.toml` | Accepted |
| [0065](../decisions/0065.md) | Attribution is generated, committed and embedded; `nvs info` is the one call | Accepted |
| [0066](../decisions/0066.md) | `expr as ?T` converts without throwing, yielding `null` on failure | Accepted |
| [0067](../decisions/0067.md) | One database API: `Core\Db` is connection-named, prepared-only and capability-gated | Accepted |
| [0068](../decisions/0068.md) | Dependencies stay current; a dependency break is absorbed, never forwarded | Accepted |
| [0069](../decisions/0069.md) | Array combination is key-type-independent | Accepted |
| [0070](../decisions/0070.md) | A duration is a literal: `30s`, `1h30m` | Accepted |
| [0071](../decisions/0071.md) | Derived codecs: an explicit attribute generates `Json\Codec`/`Db\Codec`, and a decode reports every failed field | Accepted |
| [0072](../decisions/0072.md) | `Core\Task`: concurrency is a call that returns with nothing still running | Accepted |
| [0073](../decisions/0073.md) | Scheduled work is `nvs.toml` firing a `spawn script`, with a mandatory `scope` | Accepted |
| [0074](../decisions/0074.md) | HTTP defaults are safe inbound and finite outbound, by construction | Accepted |
| [0075](../decisions/0075.md) | `Core\RateLimit`: limit what only the application knows, and name the weak tier differently | Accepted |
| [0076](../decisions/0076.md) | The runtime exports what it already measures, and a label may not be `tainted` | Accepted |
| [0077](../decisions/0077.md) | Routes are compiled, not registered, and the router stops at matching | Accepted |
| [0078](../decisions/0078.md) | `nvs.toml` reloads over a local control socket, and the extension set joins the compilation key | Accepted |
| [0079](../decisions/0079.md) | Testing is a language feature: `#[Test]` compiles to a table, every test is its own isolate, and `Core\Test` is typed | Accepted |
| [0080](../decisions/0080.md) | Novis is built to serve web applications of every kind | Accepted |
| [0081](../decisions/0081.md) | A dependency is a digest, resolution is a maximum, and a package's authority is granted one line at a time | Accepted |
| [0082](../decisions/0082.md) | Novis ships the batteries: a first-party framework, split by `rule:core-api/tier-placement`'s existing tests | Accepted |
| [0083](../decisions/0083.md) | A persistent connection is an isolate, and it is opened the way a script is spawned | Accepted |
| [0084](../decisions/0084.md) | A background job is a durable row, enqueued in your transaction and run as an isolate | Accepted |
| [0085](../decisions/0085.md) | The API document is generated while compiling, so it cannot drift from the code | Accepted |
| [0086](../decisions/0086.md) | The terminal is a sink, styling is a value, and a CLI's commands are compiled | Accepted |
| [0087](../decisions/0087.md) | An unterminated bidirectional control is rejected at every boundary, by one predicate | Accepted |
| [0088](../decisions/0088.md) | A sink is a parameter that becomes an instruction, and an unclassified one refuses | Accepted |
| [0089](../decisions/0089.md) | `nvs convert` is one rule table with two modes, and every emitted line is classified | Accepted |
| [0090](../decisions/0090.md) | `==` is the only equality operator, and comparing two disjoint types does not compile | Accepted |
| [0091](../decisions/0091.md) | A run mode is two closed values, a ceiling, and a list of defaults | Accepted |
| [0092](../decisions/0092.md) | One diagnostic record, three renderings, and the sink in force picks | Accepted |
| [0093](../decisions/0093.md) | A service is one stored argv, and the installer that stores it is a sink | Accepted |
| [0094](../decisions/0094.md) | Visibility is written at every member declaration; there is no implicit `public` | Accepted |
| [0095](../decisions/0095.md) | A name that resolves to something other than what it spells is refused, never repaired | Accepted |
| [0096](../decisions/0096.md) | A route without a declared access decision does not compile | Accepted |
| [0097](../decisions/0097.md) | The built-in server is a development server and a proxied origin, and a URL never becomes a path | Accepted |
| [0098](../decisions/0098.md) | The pipeline operator is one hole substituted at parse time, never a callable applied at run time | Accepted |
| [0099](../decisions/0099.md) | The resilient tree is the AST plus a trivia layer, `nvs-lsp` is synchronous, and an LSP answer is frozen as a `.lspt` case | Accepted |
| [0100](../decisions/0100.md) | Against Python, Novis claims the tool that gets handed over, not the script that gets thrown away | Accepted |
| [0101](../decisions/0101.md) | A `secret` value is concealed in the editor by default, the range comes from the server, and `tainted` gets no default decoration at all | Accepted |
| [0102](../decisions/0102.md) | A request is matched once, and the route table completes without crossing into dispatch | Accepted |
| [0103](../decisions/0103.md) | Configuration is a tree of files, and file ownership is the trust anchor | Accepted |
| [0104](../decisions/0104.md) | An application is an entry file path, and a per-app block is keyed on it | Accepted |
| [0105](../decisions/0105.md) | An uploaded file is a stream, and there is one way to receive it | Accepted |
| [0106](../decisions/0106.md) | Nothing a request can send terminates or wedges a worker | Accepted |
| [0107](../decisions/0107.md) | A by-reference binding is spelled `inout`, at the declaration and at the call | Accepted |
| [0108](../decisions/0108.md) | One reference index answers five features, editor completion may only offer what the compiler already derived, and a template region gets services but no second formatter | Accepted |
| [0109](../decisions/0109.md) | A `for` header may declare its own counter, and an init clause is a declaration or an expression list, never both | Accepted |
| [0110](../decisions/0110.md) | One method's repeated routes share a name when they share a path | Accepted |
| [0111](../decisions/0111.md) | A PHP built-in completes to its Novis destination, and a completion item may only insert what the registry holds | Accepted |
| [0112](../decisions/0112.md) | Authority is keyed on the enclosing namespace, and an optional capability degrades where a required one refuses | Accepted |
| [0113](../decisions/0113.md) | A qualified name is absolute, and the leading `\` does not parse | Accepted |
| [0114](../decisions/0114.md) | An array literal's own type is synthesized for one code action, and no compile path asks for it | Accepted |
| [0115](../decisions/0115.md) | The reactor reports readiness, and a stream that would block parks its own task | Accepted |
| [0116](../decisions/0116.md) | An isolate's arena is an ownership root, not an address range | Accepted |
| [0117](../decisions/0117.md) | An implemented Core member documents itself in the registry | Accepted |
| [0118](../decisions/0118.md) | A capability is checked at the door to the effect, and declared in one table | Accepted |
| [0119](../decisions/0119.md) | An expression-level `catch` is a typed arm on one guarded expression, and it lowers to the block form | Accepted |
| [0120](../decisions/0120.md) | The image component is a pipeline that crosses the boundary once, and gd is not inherited | Accepted |
| [0121](../decisions/0121.md) | PDF generation is sandboxed HTML rendering with no I/O | Accepted |
| [0122](../decisions/0122.md) | HTML parsing is a WHATWG entry on `Core\Html`, over `Core\Xml`'s tree | Accepted |
| [0123](../decisions/0123.md) | Spreadsheet reading and generation are one sandboxed component with no I/O | Accepted |
| [0124](../decisions/0124.md) | PHP 8.6 lands as four compile-time refusals and one session rule | Accepted |
| [0125](../decisions/0125.md) | A class reference is a type, and `as` is its only source | Accepted |
| [0126](../decisions/0126.md) | A property key is a checked name, and `as` is its only source | Accepted |
| [0127](../decisions/0127.md) | The end of a script is observable: `Core\Script::onExit` runs at every non-fatal ending | Accepted |
| [0128](../decisions/0128.md) | A PDF page is a decode source of the image component | Accepted |
| [0129](../decisions/0129.md) | `Core\Password::verify` reads a PHP-stored bcrypt hash, and `hash` never writes one | Accepted |
| [0130](../decisions/0130.md) | Usage telemetry is opt-in, counts only operator actions, and a serving process never uploads | Accepted |
| [0131](../decisions/0131.md) | A temporary directory dies with its script, and the runtime's sweep never throws | Accepted |
| [0132](../decisions/0132.md) | A driver is a sans-IO codec plus its own state machine over the parking stream, and the five are an enum rather than a trait | Accepted |
| [0133](../decisions/0133.md) | A launderer answers its sink's carrier, and only an idempotent escape answers a `string` | Accepted |
| [0134](../decisions/0134.md) | Every shipped feature owes four proofs, and the roster of features is derived rather than kept | Accepted |
| [0135](../decisions/0135.md) | A fixed-key shape parameter is one `CoreTy` carrying its arms, and it flattens at the ABI exactly as an options bag does | Accepted |
| [0136](../decisions/0136.md) | A `callable` carries its signature | Accepted |
| [0137](../decisions/0137.md) | A doc comment is `///`, and its only tags are `@see` and `@example` | Accepted |
| [0138](../decisions/0138.md) | a connection future is driven by the coroutine that owns it, and a waker is one wake | Accepted |
| [0139](../decisions/0139.md) | A session is a record its store issued, and its backend is never the local tier | Accepted |
| [0142](../decisions/0142.md) | A store an operator configured is authorized by the configuring, and may be a Unix socket | Accepted |
| [0143](../decisions/0143.md) | A push runs the lane its diff needs; the nightly and the release run all of it | Accepted |
| [0144](../decisions/0144.md) | `::class` answers the class a value *is*, so `static::class` and `$obj::class` are run-time reads | Accepted |
| [0145](../decisions/0145.md) | A schema is a value: `Core\Db\Schema` converges a closed vocabulary, and neither versions nor parses SQL | Accepted |
| [0146](../decisions/0146.md) | A signature is over a payload, and a URL is a payload `Core\Uri` already canonicalizes | Accepted |
| [0147](../decisions/0147.md) | An options bag tells an omitted key from a written `null`, and `null` is the one spelling that removes | Accepted |

Retired numbers, folded into the ADR that now states the rule: **0032** → [0029](../decisions/0029.md) § 1.

## Decisions taken at project start

Recorded here rather than as individual ADRs. Promote one to its own file if it is ever seriously
challenged.

**Rust as the implementation language.** Memory safety in the runtime is a product requirement, not an
implementation preference; Cranelift, the async ecosystem and the pure-Rust protocol crates all live here.

**Cranelift JIT as the only execution tier, no interpreter.** Chosen for peak performance and a single
semantics implementation to keep correct. The cost is that the first runnable program requires the whole
front end plus a working backend. Mitigated by shipping a *baseline* tier where every operation lowers to a
call into a Rust runtime helper — mechanically close to an interpreter loop, therefore quick to get
correct — with typed inlining layered on later behind the same IR boundary.

**`exit` is a fourth ABI status, not a `FATAL` carrying a code.** `nvs_runtime::EXITED` sits beside `OK`,
`THROWN` and `FATAL`, and the status `exit(n)` named rides out on the request context rather than in the
status word. A `FATAL` is a *failure* — `rule:errors/escalation-ladder`'s tier 3, reported at
the request boundary as one — while `exit(0)` is the most ordinary end a PHP program has, so folding them
together would report every clean exit as an internal error. What the two do share is propagation: neither
is catchable, because `nvs_ir::ir::Terminator::Catch` admits only `THROWN`, and **neither runs a
`finally`**, which is PHP's own behaviour for `exit` — checked against `php -r`, not assumed, and therefore
priority 2 rather than a simplification. The cost is one more constant that every status check already
handles by comparing against `OK`, and one `i64` per request. The frame's locals are still released,
because `exit` lowers to an ordinary helper call carrying `rule:errors/propagation`'s error edge. `exit("message")` is PHP's
other spelling of the same construct: the message is written and the status is `0`; anything that is
neither an `int` nor a `string` is a type mismatch at the operand, since `rule:types/conversion` has no implicit
conversion to offer there.

**SIMD is a dependency's job, and the JIT emits scalar code.** No `target-cpu` flag is set anywhere, on any
platform: LLVM autovectorizes the Rust crates at each target's *baseline* ISA — SSE2 on x86_64, NEON on
aarch64 — and no further, because a `native` build produces a binary that faults on the next machine. The
wide, feature-detected SIMD that actually earns its keep arrives through dependencies that hand-wrote it and
dispatch at runtime: `memchr` behind every `regex` prefilter ([0056](../decisions/0056.md)), `blake3`
on the artifact-cache path ([0042](../decisions/0042.md)). That is the same trade the
pure-Rust-dependency rule below already makes — the `unsafe` lives in a fuzzed crate with a user base rather
than in ours, where `unsafe_code = "forbid"` and a stable-pinned toolchain (so no `std::simd`) bar it
anyway. When a byte-scanning leaf turns out to be hot — UTF-8 validation, grapheme scanning,
[0024](../decisions/0024.md)'s HTML auto-escape — reach for such a crate, never for
`core::arch` intrinsics. Cranelift, meanwhile, has no autovectorizer: its vector instructions exist to lower
wasm's fixed 128-bit SIMD, not to be discovered from scalar loops, so JIT-compiled Novis is scalar by design.
Little is lost — a request's hot path is refcounting, ordered-hash lookups and tagged dispatch, not the
dense homogeneous loops a vectorizer needs — and vectorizing a `float` reduction would reassociate its
additions, which the priority ordering's rank 2 forbids outright. Two things to know before anyone claims a
win: [0026](../decisions/0026.md)'s instruction counts flatter SIMD, because
callgrind does not model vector port throughput, and Cranelift's own vector support is shaped by wasm's
fixed 128-bit SIMD, which caps anything built on it there regardless.

**Thread-per-core, shared-nothing runtime.** One single-threaded executor pinned per core; a request is
assigned to a core and never migrates. This is what makes value refcounts *non-atomic* (a heap is only ever
touched by one thread), makes cross-request state contamination structurally impossible rather than merely
prevented, and still uses every core — parallelism comes from N independent executors. Compiled code is
immutable and therefore shared across all cores through `Arc` with no copying.

**Stackful coroutines for suspension.** Validated by spike #3, now the guard tests
`a_helper_can_suspend_with_jit_frames_live_above_it` and `a_coroutine_round_trip_stays_cheap` in
[`benches/abi-probe`](../../benches/abi-probe/). The decisive property is the absence of *function
colouring*: any Novis function may perform I/O and yield without being marked `async`, so converted PHP call
chains become concurrent with no rewriting. The cost is a stack per in-flight task (default 64 KiB,
configurable, grown lazily) and a small audited unsafe core for stack switching, taken as a dependency
(`corosensei`) rather than hand-rolled. The memory is paid deliberately, under
[0004](../decisions/0004.md).

**Isolated workers for CPU parallelism.** Work dispatched to another core gets its own heap; values
crossing the boundary are deep-copied, or moved when the refcount is 1. Data races are impossible by
construction rather than by discipline, which is what lets the refcounts stay non-atomic. The same rules
govern the script-level boundary in [0006](../decisions/0006.md), deliberately: one set of
value-crossing rules, not two — and [0023](../decisions/0023.md) gives that one
rule its formal definition, shared with `serialize()`/`unserialize()`.

**Strict shared-nothing requests.** Only compiled code survives a request. The consequence — reconnecting
to the database every request — is accepted for v1; `nvs-host` reserves an unused `PersistentRegistry` seam
so pooling can be added later without redesign. The same isolation is reachable from inside the language:
`spawn script` runs another `.nvs` file as a child isolate of the request tree, and an inbound request is
simply the root isolate of its tree, so both paths are one implementation.

**Safepoints emitted from the first backend commit.** A poll at every loop back-edge and function entry is
the single mechanism behind CPU-time limits, client-disconnect cancellation, the cycle collector, the
profiler, debugger breakpoints and later deoptimisation. Retrofitting it would mean rewriting codegen, so
it is not deferrable.

**Server-level configuration, not per-project.** `nvs.toml` is root-owned, TOML
([0064](../decisions/0064.md)), and per-app capability blocks live in the *root* config so an
application can never grant itself rights. What a script may change about its own configuration at runtime
is per-directive and is argued in [0005](../decisions/0005.md).

**Pure-Rust dependencies by default.** A memory-safe runtime cannot contain arbitrary C. Deviations are
explicit, argued and few — currently only SQLite (`rusqlite`), where no credible pure-Rust implementation
exists. The admission test is [0051](../decisions/0051.md) § 4's.

**Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours.**
The neighbouring question to the one above — not *what may we depend on*, but *what may we write ourselves*.
Anything with an external specification (a protocol, a parser, a wire format, a cipher, a codec, a cron
expression, a timezone database) is a dependency, and **if no first-class crate exists, the feature is not
built** — a second-rate implementation of somebody else's specification is a security surface we would then
own forever. Anything about *Novis's own* compiler or scheduler has no possible crate and is ours by nature.
It is the same split the project already lives with: Cranelift compiles, and the IR lowering into it is
ours; `serde_json` parses, and the derive that emits Novis IR from Novis types
([0071](../decisions/0071.md)) could not be a crate if we wanted it to be. When a feature is half of each,
say which half is which before writing either.

**Checked-return call sites go through one code path.** See [0002](../decisions/0002.md): a missing
status check would silently swallow an exception, so no caller constructs a raw `call` instruction.

**`unsafe` is confined to named crates, each declaring its own policy.** The workspace sets
`unsafe_code = "forbid"`; crates that genuinely need it opt down to `deny` and allow individual blocks with
a stated reason. Currently `nvs-runtime`, `nvs-codegen`, `nvs-stdlib` and `benches/abi-probe`, which must
call JIT-compiled code to measure it. The probe is `publish = false` and is not a dependency of anything
shipped, so it does not widen the runtime's unsafe surface.

**`continue` inside a `switch` continues the enclosing loop.** PHP counts a `switch` as a looping structure
for `continue`, so a bare one there behaves as `break` — and PHP has warned since 7.3 that you probably
meant `continue 2`. Novis takes the meaning that warning points at: `switch` owns `break` and nothing else, so
`continue` always means the innermost enclosing loop. The alternative is a keyword that silently means one
thing inside a `switch` and another everywhere else, which the priority ordering's simplicity rule refuses
to buy for a compatibility PHP itself discourages. `nvs_ir::lower::Lowering::lower_switch` implements it and
`tests/differential/lang/a-switch-and-a-match-agree-with-php.nvst` pins it against PHP's `continue 2`.

**A written level counts the way PHP counts, and `continue` then walks outward.** `break N`/`continue N`
name the `N`-th enclosing statement, and a `switch` is one of them for *both* keywords — that is PHP's rule,
and departing from it would silently retarget `continue 2` inside a `switch` inside two nested loops from
the inner loop to the outer one, which is the one outcome worse than a diagnostic. A level that lands on a
`switch` frame then looks further out for a loop, which is the paragraph above generalized from the bare
keyword to a written level: it makes `continue 2` inside a `switch` mean in Novis exactly what it means in
PHP, and leaves the divergence exactly where it already was — a level naming a `switch` for `continue`
continues the loop rather than breaking the `switch`. A level with nothing to name is
`E0475` (`nvs_types::locals::check_exit_level`), never a panic: a non-literal level, a `0`, a level past the
enclosing depth, and `continue` with no loop at or outside its frame. PHP refuses all four at compile time
too. `nvs_ir::lower::Lowering::lower_break`/`lower_continue` lower the rest, and
`tests/conformance/lang/a-break-leaves-the-level-it-names.nvst` pins the whole file byte-for-byte against
PHP's output.

**A ternary's or a `match`'s branches join at the union's erasure, and widen at the binding.** Two branches
that lower to two representations are not reconciled by promoting one into the other: the checker has
already typed the whole expression as the *union* of its branches, and `nvs_ir::lower`'s `erase_checked_ty`
erases a union whose members do not share a representation to the tagged one, so the phi carries that and
`nvs_ir::lower::Lowering::join_representations` tags each branch in its own block. This is the same line
integer `/` draws and for the same reason ([0007](../decisions/0007.md) §§ 2 and 4): § 4's promotion
rows belong to an *operator*, whose result type that table fixes, and § 2's implicit `int`→`float` widening
happens at a `float` **position** — a binding, a parameter, a `return`. A ternary branch is neither, so
`$c ? 1 : 2.5` keeps PHP's answer on its truthy path (an `int`, not `1.0`, and exact past 2^53 where the
widening would have thrown) and `float $x = $c ? 1 : 2.5;` widens exactly once, where the declared type is.
An **arm-less** `match` is refused where it is written, `E0476`, rather than lowered: PHP parses one and
throws `UnhandledMatchError` on every evaluation, so no program that ran is lost, and a `match` is an
expression — one whose every path throws has nothing for the position it sits in to bind, pass or return,
and no value for a merge phi with no incoming edge to carry.

**A method call needs a class label, so an erased receiver is refused rather than dispatched.** `object` is
[0007](../decisions/0007.md) § 3's opaque top of every class type, and it erases to exactly the
pointer a named class does — `nvs_ir::lower`'s `erase_checked_ty` maps `CheckedTy::Object` and
`CheckedTy::Shape` onto the same `Ty::Object` a `CheckedTy::Class` gets, so nothing below the checker ever
wanted the label for *representation*, and `object` is a declared type in every position a class name is.
What does want it is *resolution*: `$o->m(...)` has no signature to check its arguments against and no
return type for the position it sits in. [0036](../decisions/0036.md) § 4 already answered the
**property** half of an erased receiver — a name-keyed runtime fetch, and a write checked against the
field's real declared type — and stopped at properties on purpose. The call half is therefore `E0477` where
it is written (`nvs_types::expr::calls::report_method_on_erased_receiver`), naming the two narrowings that
do resolve: `instanceof` proves the class inside the guarded branch, and `as ClassName` converts to it or
throws. There is no `__call` to fall back on ([0014](../decisions/0014.md)), and § 3's "a dynamic call
with runtime-checked arguments, at `mixed`'s cost" is deferred for `callable` and was never granted to
`object`. **It is one code across every receiver that names no class**, because it is one mistake and the
resolution it fails is the same one: a union naming no single class, an intersection, and the types that
can hold no object at all — a scalar, an `array<T>`, a `void` call's result — all take `E0477` too, with
only the help splitting (narrow it, or convert it, or nothing at all for a value that does not exist).
That is where the call half parts company with the property one, which splits a *deferral* off from
`E0495`: a property read through an erased receiver has a name-keyed fetch to defer to and a call has
nothing. `mixed` is the one receiver deliberately left out — [0007](../decisions/0007.md) § 2 makes
it the one unchecked position, so it defers rather than refuses, and until that lowering exists it is the
one shape `nvs-ir`'s own panic at `lower/expr.rs` still names; the paragraph below owns *how* it is
answered. Refusing is the reversible half of that pair: a later decision can turn this diagnostic into
dispatch, while a program that already dispatched could not be taken back.

**A call through a `mixed` receiver is marshalled by the receiver's own descriptor, not by a per-method
thunk.** [0036](../decisions/0036.md) § 4 grants the deferral and says nothing about the
convention, so this paragraph is its home. Almost nothing has to be marshalled at all, which is the fact the
design turns on: [0002](../decisions/0002.md) makes **one** calling convention normative for every
call, so `nvs_runtime::abi::NvsFn` is already a context, an array of 16-byte tagged `Value`s and one tagged
`out` slot, and `nvs-codegen`'s `store_value`/`load_value` already write each argument and each return
*with* its tag while a typed callee reads only the payload half. A site holding tagged values therefore has
tagged slots to fill, and the answer comes back tagged for the `mixed` the call's own type is — no
conversion in either direction, and a narrower binding takes `as T` exactly as one holding a `callable`'s
result does. What is missing is not the marshalling but the callee's **declared shape**: how many parameters
it takes and which tag each one requires, without which the callee reinterprets slot *i* at its own
representation and an `int` handed to a `string` parameter is an arbitrary dereference rather than a fault —
the identical hole `nvs_runtime::closure`'s own module docs describe for `callable`, arrived at from the
other side. So the method row on `nvs_runtime::ClassDesc` carries them, the way a closure object already
carries `FN_ARITY` and `FN_PARAM_TAGS`: the same nibble word, the same `CLOSURE_PARAM_TAG_ANY` for a
parameter whose representation *is* a tag, and `check_param_tags` as the one implementation both paths
share, so [0007](../decisions/0007.md) § 2's `int`-into-`float` widening is not written down a
second time to be got wrong differently. They are resolved once per class in `ClassTable::set_methods`, the
precedent `ClassDesc::renderer` and `ClassDesc::unwind` set, and cost a word and a byte per method per class
**once per process** — nothing per instance and nothing per call.

The alternative on the table was a tagged-ABI thunk per method, and it loses on three of the priority
ordering's five at once: `nvs-codegen` would emit the tag rules a second time, where a safety check wants
one implementation and not two (rank 1); a thunk is a second frame on the erased path and still needs the
same name lookup to be found at all, so it buys no dispatch (rank 3); and it spends a whole compiled
function per method in every unit whether any `mixed` receiver exists or not, against sixteen bytes on a
descriptor (rank 5). The statically typed path pays nothing either way and keeps its fixed label; the erased
path pays a tag test on the receiver, one binary search of the flattened method table by name, and a shift,
a mask and a compare per argument. Every failure a program can reach is a catchable throw on
[0002](../decisions/0002.md)'s error edge and never a fault, worded as the diagnostic that names the
same mistake where a static type shows it — a receiver whose tag is not an object (`E0477`'s reading), a
class whose table has no such name (`E0405`'s), and a count or a tag the callee does not admit (`E0402`'s
and `E0401`'s) — so one mistake reads one way whichever end sees it. Three shapes are answered by that
throw rather than by dispatch, each because the row cannot describe them and not as a rule about erasure: a
**non-`public`** member, since a `mixed` receiver is outside every class by construction and the row carries
the visibility bit that says so; a **variadic or `inout`** parameter list, which is packed and written back
at the *call site*, the limit `E0721` already names for [0043](../decisions/0043.md)
§ 4's synthesized forward; and a **`Core`**-owned class, whose members are native symbols that *borrow*
argument 0 where a compiled method owns its parameters — the very difference that made `renderer` its own
descriptor field rather than a row in the table, and reaching them from here wants a second field per
member that no case asks for yet.

**A closure parameter naming a class is checked against the argument's own ancestry, at the closure's
entry.** `nvs_ir::lower::param_tag_nibble` gives every class name — and `object`, and a shape — the same
nibble 7, a nibble naming a representation and four bits having no room for a label, so
`nvs_runtime::closure::check_param_tags` refuses a `string $c` handed a `Core\Cli\Text` and would accept
an unrelated `Marker $c` handed the same value. That acceptance was the **only** way a named-class binding
came to hold an instance of another class: every other position is checked where it is written, and
[0036](../decisions/0036.md) § 4's erased receiver carries no label at all and therefore defers
to a name-keyed fetch. A binding that *does* carry a label is read and written at a fixed offset, so the
lie is a type confusion rather than a wrong answer — two `final` classes and one wrong `Core\Arr::filter`
callback wrote an `int` over a `string` field and the next read dereferenced it — which is
[0004](../decisions/0004.md)'s priority 1 and not a matter of taste.

Two boundaries could pay, and the cheaper one is not the safer one's equal. Making every named-class
property access name-keyed would close it everywhere and spend priority 3 in every program, most of which
never write a closure at all. Checking the argument against the parameter's declared class **at the
closure's entry** spends one `nvs_object_instanceof` — one flattened linear scan of the ancestry
(`nvs_runtime::object::NvsObj::is_instance_of`) — per class-declared parameter per call, and only in the
position where nothing else looked. That is what `nvs_ir::lower::closure::check_param_class` emits: the
body's first block branches on the same `instanceof` `$x instanceof C` lowers to, and the miss throws a
`LogicError` in the sentence shape `check_param_tags` already writes. The tag word is unchanged and gains
no class channel — a per-closure-instance list of descriptors would spend an allocation at every closure
literal to answer a question the body's first block asks for free. Both lines are pinned from Novis by
`tests/conformance/core/out-a-callback-parameter-naming-a-class-checks-the-argument-class-at-entry.nvst`:
the tag word's around objecthood, the entry check's around ancestry, so a parent class and an implemented
interface both accept the instance an exact class does.

Three things the entry check deliberately does not do. A **`?C` parameter is unchecked on both lines**,
erasing to `nvs_ir::Ty::Tagged` before any class survives — the same nothing `mixed` gets, and for the
same reason. A **`Core` class** parameter keeps the objecthood-only check: a unit's class table is
`nvs_types::layout`'s declared tree, so there is no descriptor to compare against, and `instanceof` over a
`Core` class is not a spelling the checker admits either (`E0496`) — the only argument that could exercise
the row is the carrier a `Core` member hands its own callback, which is already of that class. And the
refusal **does not name the class that arrived**: no IR instruction reads an object's class name, so
widening `must be of type Marker, another class given` to PHP's `…, App\Holder given` means a new value
shape in `nvs-ir` and `nvs-codegen`, worth taking when something other than a message wants one.

**An `inout` parameter belongs only to a frame the call site outlives.** A by-reference parameter is a
contract between the two ends of one call: `nvs_ir::lower::call` stages a cell at the site, hands the callee
its address, and copies back when the call returns — sound precisely because the callee's frame dies first.
Two declarations break that ordering, and both are refused where they are written rather than lowered. A
**generator** inverts it outright: calling one runs none of the body, it allocates the state object and
returns ([0053](../decisions/0053.md) § 4), so the staged cell is gone before the first
`advance()` while the parked frame would still be addressing it — `E0492`,
`nvs_types::check::check_generator_by_ref_params`. A **closure** has no call site that could stage anything:
§ 2's by-value capture lets it outlive every frame in scope where it was written, so there is no frame whose
death the copy-back could be ordered against — `E0493`,
`nvs_types::expr::calls::report_by_reference_parameter`. A written signature
([0136](../decisions/0136.md)) does not reopen it: that gives a site a parameter list to
read and still no call site at which to stage a cell. Neither is a lowering we
chose not to write: there is no representation either could keep instead, because copying the value in would
stop being a reference, which is the whole observable point of `inout`. The replacements are the ones those
ADRs already name — for shared mutable state, § 2's ordinary object captured by value; for a generator,
taking the value and `yield`ing what the body computes from it. **Capturing** an enclosing `inout` parameter
is a different question and is *not* refused: § 2's capture is by value, so what it owes is a snapshot of
the cell's value at the literal, which `nvs-ir` has not written yet (its gap 9). **The spelling is
[0107](../decisions/0107.md)'s** — `inout` before the type and
again at the call site, `&` rejected in every by-reference position — and this paragraph states the rule in
it; the tree still spells it `&` until that ADR's M4 items land.

**Architecture assumptions are tested, not remembered.** Several decisions here rest on how Cranelift,
`corosensei` and Wasmtime behave rather than on our own code, and a dependency bump can invalidate them
silently. `benches/abi-probe/` checks them on every CI run, including the *premise* of
[0002](../decisions/0002.md) — that native unwinding through JIT frames is unavailable — so if that
ever changes we are told rather than left paying for a workaround that is no longer needed. It also guards a
premise about the *platform we are replacing*: that an OS process costs orders of magnitude more than a
task, which is the whole cost argument for [0006](../decisions/0006.md).

**A `static` property's storage is the request's, not the process's.** One slot per declared static per
in-flight request, armed from the declaration's own initializer when the request's `nvs_runtime::Ctx` is
built and released when it is dropped. The priority ordering settles it at rank 1: a process-global static
is a channel from one request into the next, so a token cached in one is readable by whoever sends the
next request, and no amount of care in user code closes that. `nvs run` cannot tell the two apart — a CLI
run is one request — so the divergence is invisible until `nvs serve` at M7, which is exactly when the
wrong default would have become expensive. What it costs is the PHP idiom of a process-lifetime memo,
which [0006](../decisions/0006.md) had already removed by making a script's world
per-execution; a cache that must outlive a request is a `Core` capability with a stated lifetime, not a
class variable. Because a static therefore has no constructor to assign it, its declaration must carry an
initializer unless its type admits `null` (`E0409`), and `static::$prop` — which PHP re-resolves against
the *called* class — is refused rather than silently answering the writing class's slot (`E0499`).
`nvs_runtime::ctx`'s module docs own the mechanism and what it spends.

**A diagnostic band is two digits wide, and a full one continues in a new band rather than running past
its end.** `E04xx` — types — filled at `E0499`, and the max-plus-one rule would have yielded `E0500`,
whose own digits read as `E05xx`: IR and codegen. A code is a promise that its number alone says which
stage produced it, and `nvs_diagnostics::code`'s legend table is where that promise is written down, so
`E0500` is never issued and the types band continues at **`E07xx`**, one more row in that table, opening
at `E0700`. Both alternatives cost more than a second range: widening every band to three digits
renumbers two hundred released codes and every `.nvst` case that names one, and filling the lowest hole
inside `E04xx` reuses a retired number, which the same promise forbids. `tools/brief.py` reports a band
whose max-plus-one would leave it as **full** rather than handing out the number past its end, so the
next session reads this decision off the tool instead of re-deriving it. `E08xx` was held unallocated for
whichever band filled next, and types is what filled it — a second time, at `E0799` — so
[0136](../decisions/0136.md) opens **`E08xx` at `E0800`** for the diagnostics a
written `callable` signature needs, a third row in that legend table for the one stage. `E10xx` is the
reserve that replaces it, `E09xx` being internal compiler errors.

**A `class`, `interface` or `enum` is declared at file scope, or not at all.** PHP declares a nested type
when the statement *runs*, so `if ($legacy) { class Session { … } }` makes the very existence of a name a
run-time fact. Novis resolves every type name against a static table built before any code runs — the
`require`/autoload graph of [0021](../decisions/0021.md), then `nvs_hir`'s member and
hierarchy tables, then `nvs_types::layout`'s field offsets — and none of those has a reading to give a
name that may or may not exist yet. So a declaration written inside a method body, a property hook, a
closure body or a nested block at file scope is `E0233` where it is written, reported by
`nvs_types::locals`' per-body walk, which is reached only from inside a body and therefore needs no
"am I nested" test of its own. Both cheaper readings were rejected: declaring it unconditionally at file
scope silently changes the program PHP wrote, and admitting a conditional entry in the class table would
put a run-time question inside every name resolution, layout and dispatch decision below it — priority 4,
paid for once here rather than at every later lookup. What it costs is PHP's conditional-class idiom,
whose two real uses — a polyfill and a feature switch — are `require` of one file or the other, which is
static and already works. An anonymous class — PHP's `new class { … }` — is the same nested declaration in
expression position and is refused the same way: it has no name for the static table to hold, and its two
real uses, a one-off implementation of an interface and a test double, are a named class in the same file
or a closure ([0031](../decisions/0031.md)).

**A `try` has at least one `catch` or a `finally`, and a `catch` clause names exactly one class.** PHP
refuses a bare `try { … }` too, and the parser here accepted it only by omission. `catch (A | B $e)` is
refused: `$e` carries one static type ([0007](../decisions/0007.md) § 1's `catch` row), and a body
shared by two classes is written as two clauses or as one clause on their common ancestor — the shape
[0119](../decisions/0119.md) § 4 already requires of
an expression arm. Both are parse-time refusals in the rejected-PHP band, and each names its rewrite.

**A class or interface constant writes its type.** PHP 8.3's untyped `public const LIMIT = 9;` parsed
here and took the type of the value it folded to. That reading is a guess the declaration never made,
and it is not available everywhere the form is: an interface constant records no value for `nvs-ir` to
inline and panicked the lowerer at the first use, and a value with no constant form at all (`[1, 2]`)
has nothing to read a type from either. Every other binding
[0007](../decisions/0007.md) § 1 governs — property, parameter, return, local, `foreach`,
`catch` — already writes one, so the omission was the single hole in that rule, and closing it removes
both panics by construction rather than one shape at a time. It is `E0246` at the `const` keyword, in
the rejected-PHP band, and the rewrite is the type the value already has. What it costs is one word
per constant in a migrated file; what the inference did is now a check against what was written.

**`namespace X;` is the only namespace statement — once per file, before any declaration.** The braced form
`namespace X { … }`, and with it a file holding two namespaces, is refused at parse time.
[0112](../decisions/0112.md) keys authority on the namespace enclosing the
code and [0104](../decisions/0104.md) keys an application on its entry file; a file
that is two namespaces is a file whose authority is a function of the line number, and nothing below either
ADR has a reading to give that. The rewrite is one file per namespace.

**One `use` names one import; PHP's group form is refused.** `use App\Models\{User, Post};` is `E0238`
where the `\{` is written, sibling to [0015](../decisions/0015.md) § 2's `E0212` on the other half of
the same statement. That ADR's rule is that a name is reachable under exactly the spelling it was declared
with, and the group form does not break it — every short name it introduces is still that name — so this
is priority 4 rather than priority 2: a second spelling of a statement that already exists, whose payoff
is fewer lines and whose cost is that the set of short names a file introduces is no longer a `grep` for
`^use` returning one name per line. Novis takes the line-per-import reading, the same trade the alias
refusal already made. The parser reports and then skips the brace group, so the statement still yields a
`UseDecl` for the prefix that was written and nothing downstream sees a half-parsed import.
