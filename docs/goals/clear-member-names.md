---
looper: 1
title: "Every member that does something is named with a verb, and every API name says what it does"
summary: "Reached when the naming rules require a verb for every member with an effect, a copy-form name for every member that returns a changed copy, a predicate for every bool read and full words everywhere, bun nv names --check enforces them over Core and every first-party package, every member that broke them is renamed with a moved-name row and fix, every caller in the tree uses the new names, and the rules, the reference, the examples, the feature proofs and the website say so."
context: [docs/rules/core-api/verb-lexicon.md, docs/rules/core-api/members-are-full-words.md, docs/rules/core-api/a-moved-name-says-where-it-went.md, crates/nvs-stdlib/src/moved.rs]
stages:
  - id: 1
    title: "the naming rules are rewritten, and bun nv names --check lists every name that breaks them"
    status: open
    context: [docs/rules/core-api/shape-rules.md, docs/rules/core-api/symmetric-names.md, crates/nvs-stdlib/src/registry.rs, tools/nv/cmd/ci.ts, tools/nv/renderers/website-core.ts]
  - id: 2
    title: "a moved instance or package member gets its new name and a fix, and bun nv api-rename sweeps the tree"
    status: open
    context: [crates/nvs-types/src/moved.rs, crates/nvs-syntax/src/parser/expr.rs]
  - id: 3
    title: "the request path is renamed: Response, Request, Sse, Task, Metrics, Router\\Match and Script"
    status: open
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/sse.rs]
  - id: 4
    title: "the command line and the file system are renamed: Cli, Test, Xml\\Writer, IO and Path"
    status: open
    context: [crates/nvs-stdlib/src/cli.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/path.rs]
  - id: 5
    title: "values and data are renamed: BigInt, Decimal, Math, Arr, ObjectSet, Uri, Time, Db, Http and Ldap"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs, crates/nvs-stdlib/src/db/registry.rs, crates/nvs-stdlib/src/ldap/flags.rs]
  - id: 6
    title: "the first-party packages are renamed: Novis\\Image, Novis\\Pdf and Novis\\Spreadsheet"
    status: open
    context: [extensions/image/nvs/Image.nvs, extensions/pdf/nvs/Document.nvs, extensions/spreadsheet/nvs/Sheet.nvs]
  - id: 7
    title: "every further name the check lists is fixed, and no old name is left in the tree"
    status: open
  - id: 8
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [website/src/data/changes.json]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: names-tool
    stage: 1
    name: "bun nv names judges each naming rule on a fixture and names the member and the rule it breaks"
    run: [bun, test, tools/nv/test/names.test.ts]
    tests: [a void member without an effect verb is named, a bool member without a predicate is named, a member returning a changed copy needs a copy name, a member changing its object in place needs an effect verb, a class returning itself must be classified, a truncated word is named, a new noun member with an argument must be listed as a read]
    timeout: 900
  - id: moved-members
    stage: 2
    name: "an old instance member and an old package member each get their new name and a fix"
    run: [cargo, test, --lib, a_moved_]
    tests: [a_moved_instance_member_gets_its_new_name_and_a_fix, a_moved_package_member_gets_its_new_name_and_a_fix, a_moved_spawn_option_gets_its_new_name]
    timeout: 1800
  - id: api-rename-tool
    stage: 2
    name: "bun nv api-rename rewrites qualified names, lists unresolved instance calls and checks for old names"
    run: [bun, test, tools/nv/test/api-rename.test.ts]
    tests: [a qualified static call is rewritten, a short name brought in by use is rewritten, an instance call outside a project is listed and not rewritten, check fails naming each old name left]
    timeout: 900
  - id: names
    stage: 7
    name: "every Core and first-party package name follows the naming rules"
    run: [bun, nv, names, --check]
    timeout: 900
  - id: no-old-names
    stage: 7
    name: "no name this goal moved is written anywhere in the tree but the moved table and the changes entries"
    run: [bun, nv, api-rename, --check]
    timeout: 900
  - id: proofs
    stage: 8
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, clear-member-names]
    timeout: 1800
  - id: close
    stage: 9
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, clear-member-names]
    timeout: 900
---

# Every member that does something is named with a verb, and every API name says what it does

## Why

The user asked for it on 2026-11-24, after a review of how a framework built on Novis names its builder
methods. `Form->field($f)` adds a field, but at the call site it reads like a read. The same review of
Novis's own API found the same problem in `Core` and in the first-party packages.

**The rules cover reads, not actions.** `rule:core-api/verb-lexicon` fixes what `is…`, `has…`, `find…`
and `count…` return. It lets a stateful object "use" `set…` and `add…`, and never requires a verb.
So `Core\Response::json($v)` sends the response, while `Core\Request::json()` reads the body.
`Novis\Spreadsheet\Sheet->chart(...)` adds a chart, and `Sheet->cell(...)` in the same class only returns an
address. `Novis\Pdf\Document->password($p)` returns a copy with a password, and nothing at the call site
says whether it reads, sets or copies.

**The guards check less than the rules claim.** `members-are-full-words` and `verb-lexicon` both name
`crates/nvs-stdlib/src/registry.rs` as their guard, but its only naming tests are the `try…`/`…OrNull`
ban (`registry.rs:4946`) and casing. `Core\BigInt` has `sub`, `mul`, `div`, `mod`, `neg`, `shl`, `shr`,
`pow` and `powMod`, which the full-words rule does not allow, and nothing failed. The first-party
packages have no naming rule at all.

The user's calls: do it properly, and breaking every caller is expected while Novis is pre-alpha
(`rule:core-api/no-deprecation-window-before-a-release`). Everything below is the session's design,
built on unless the user picks otherwise.

## Standing decisions

**The rule, in four parts.** It replaces the body of `docs/rules/core-api/verb-lexicon.md`, and it holds
for `Core` and for every first-party package (`extensions/*/nvs/`, `packages/rest/`).

1. **A member with an effect starts with a verb.** An effect is anything beyond returning a value: it
   changes its own object, sends or writes output, changes the request, the response, the session, a
   file, the network or the clock, or prompts the user. `add…` adds to a collection, `set…` replaces one
   value, `send…`, `write…`, `start…`, `open…`, `run…`, `show…`, `ask…` and `create…` say what they do.
   `on…` registers a handler.
2. **A noun or an adjective names what a member returns, and such a member has no effect.** It reads a
   property (`Request::path()`), reads by key (`$row->int('id')`), computes a value
   (`Str::length($s)`, `$a->gcd($b)`), or builds a new value (`Duration::seconds(5)`, `Zone::of(…)`).
3. **An instance member that returns a changed copy says it is a copy.** It is named `with…`,
   `without…`, `plus`, `minus`, `keeping…`, or a past participle (`negated`, `multipliedBy`,
   `resized`, `signed`). The plain imperative (`add`, `resize`, `sign`) is kept for a change in place.
   So `$set->add($x)` changes the set, and `$image->blurred(2)` returns a new image. A call whose
   result is thrown away now reads wrong at a glance. Static members are not held to this: a static
   takes its subject as an argument, and `rule:core-api/nothing-mutates` already says it changes nothing,
   so `Str::trim($s)` keeps its name.
4. **A `bool` read starts with a predicate:** `is…`, `has…`, `can…`, `must…`, `needs…`, `allows…`,
   `uses…`, `contains`, `startsWith`, `endsWith`, `equals`, `matches`, `exists`, `any`, `all`. A member
   with an effect that also returns `bool` keeps its effect verb (`Config::set`, `Queue::cancel`).

The existing half stays: `find…` returns `?T`, `count…` returns `uint`, `to…`/`from…` convert, and the
`try…`/`…OrNull`/`…Safe`/`…Ex` ban stays in `registry.rs`.

**One word keeps one meaning across `Core`.** A word used as a verb with an effect is not also used as a
noun read in another class. That is why `Request::query` and `Request::post` move: `Db\Connection->query`
runs a statement and `Http\Client::post` sends a request.

**Full words, with three closed lists.** `members-are-full-words` gains its lists in
`tools/data/api-names.json`, and the check reads them from there:
- **Math names**, allowed as written: `abs`, `min`, `max`, `sqrt`, `cbrt`, `exp`, `log`, `sin`, `cos`,
  `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `hypot`,
  `gcd`, `lcm`, `ceil`, `floor`.
- **Acronyms**, which are the standard's own name and count as one word: `json`, `html`, `xml`, `uri`,
  `url`, `uuid`, `sql`, `tls`, `svg`, `cpu`, `mime`, `hmac`, `pid`, `dn`, `rdn`, `sid`, `rid`, `cc`,
  `bcc`, `hex`, `rgb`, `rgba`, `iso`, `ip`, `mac`, `tty`, `cdata`, `csv`, `des`, `mns`, `giro`, `cldr`, `v4`, `v7`.
- **Banned truncations**, each named with its full word in the check's message: `dir`, `args`,
  `params`, `param`, `nanos`, `sub`, `mul`, `div`, `mod`, `neg`, `shl`, `shr`, `pow`, `fdiv`, `diff`,
  `dirname`, `basename`, `cwd`, `stat`, `auth`.
`info`, `stats` and `app` are dictionary words and stay, so no class is renamed (`Reflect\ClassInfo`,
`Queue\Stats`). The lists cover member names, option field names and the class names of the packages.

**The check: `bun nv names --check`.** A new command, `tools/nv/cmd/names.ts`, with its tables in
`tools/data/api-names.json`. It reads `nvs meta --json` from the built binary, which has `Core` and the
extensions, and from `packages/rest/`. It fails, naming the member and the part of the rule, when:
- a member returning `void` does not start with an effect verb;
- a member returning `bool` starts with neither a predicate nor an effect verb;
- an instance member returning its own class is in a class listed as `value` and is not named as a
  copy, or is in a class listed as `changing` and does not start with an effect verb. A class with
  such a member that is on neither list fails, so every new builder is classified once by its author;
- a word of a name is a banned truncation, or a math name is used outside a whole member name;
- a member takes an argument besides its subject, starts with no verb, and is not on the `reads` list.
  That list is every such member that exists after the renames, and a new one fails until its author
  adds it. This closes what the types cannot show: a noun with an effect that returns another type,
  such as `Workbook->sheet($name)`.
The effect verbs, predicates and copy words are lists in the same file. A past participle (a word
ending in `ed` or `en`, or one of the irregular forms listed) counts as a copy word. The command joins
`bun nv ci --check` beside `lint tables` (`tools/nv/cmd/ci.ts:332`), so CI and `bun nv close` run it.
Both rule fragments' `guardedBy` (`data/rules/core-api/verb-lexicon.json`,
`data/rules/core-api/members-are-full-words.json`) name `tools/nv/cmd/names.ts` beside `registry.rs`.

**Every rename gets a moved row and a fix.** `crates/nvs-stdlib/src/moved.rs:70` gets one
`Move::renamed` row per `Core` rename, with `since` set to the next release, as the rows written after
the first release set it. Stage 2 extends the moved pass (`crates/nvs-types/src/moved.rs:40`) to an
instance call on a typed `Core` receiver and to the first-party package classes, so `$uri->resolve(…)`
and `$sheet->chart(…)` get `E0405` with the new name and a fix. If the table's test cannot see a
package's members from the registry, package rows are checked by `bun nv names` instead, which reads
them from `nvs meta --json`.

**The sweep: `bun nv api-rename`.** A command, because the same rewrite runs in four stages. It reads
this goal's rows from `moved.rs` and:
- rewrites every `Class::old(` written in full, and every short name a `use` brings in, in every text
  file of the tree (`.nvs`, `.nvst`, `.lspt`, `.md`, `.rs` and `.ts` strings, website pages and snippets);
- runs `nvs check --fix` over every project directory, which fixes instance calls by type;
- lists each `->old(` it could not resolve, with `file:line`, for the session and its subagents to fix
  by reading;
- with `--check`, exits non-zero and names every old name still written outside `moved.rs` and
  `website/src/data/changes.json`.

**The renames.** This table is the list. The new name keeps the arguments unless a note says otherwise.

| Old | New | Why |
|---|---|---|
| `Response::json`, `html`, `text`, `bytes` | `sendJson`, `sendHtml`, `sendText`, `sendBytes` | effect; matches `sendFile` |
| `Response::slotted()` | `sendSlotted()` | effect |
| `Response::stream(…)` | `startStream(…)` | effect |
| `Request::query`, `queryArray`, `queryAs` | `queryParameter`, `queryParameters`, `queryParametersAs` | one word, one meaning; matches `Uri->withQueryParameter` |
| `Request::post`, `postArray`, `postAs` | `formField`, `formFields`, `formFieldsAs` | one word, one meaning |
| `Sse::stream()` | `startStream()` | effect |
| `Sse->retry($after)` | `setRetryDelay($after)` | effect |
| `Metrics::gauge` | `setGauge` | effect; its siblings are verbs |
| `Task::afterResponse` | `runAfterResponse` | effect |
| `Router\Match->params()`, `param($name)` | `parameters()`, `parameter($name)` | truncation |
| `Script::args()` | `arguments()` | truncation; spawn's `with(args: …)` and `upgrade`'s `$args` become `arguments` with it |
| `Cli::secret`, `live`, `progress` | `askSecret`, `showLive`, `showProgress` | effect |
| `Test::scriptAnswers` | `answerCli` | effect; matches `answerHttp` |
| `Xml\Writer->content`, `attribute`, `comment`, `cdata`, `instruction`, `doctype` | `writeText`, `writeAttribute`, `writeComment`, `writeCdata`, `writeInstruction`, `writeDoctype` | effect |
| `IO::temporaryDir()` | `createTemporaryDirectory()` | effect; truncation |
| `IO::makeDir`, `removeDir`, `isDir`; `IO\Metadata->isDir` | `makeDirectory`, `removeDirectory`, `isDirectory`; `isDirectory` | truncation |
| `IO::stat` | `metadata` | truncation; it returns `IO\Metadata` |
| `Path::dirname`, `basename`, `fromCwd`, `thisDir` | `parent`, `fileName`, `fromWorkingDirectory`, `thisDirectory` | truncation |
| `BigInt->add`, `sub`, `mul`, `div`, `mod` | `plus`, `minus`, `multipliedBy`, `dividedBy`, `remainder` | copy form; truncation |
| `BigInt->pow`, `powMod`, `neg`, `shl`, `shr` | `raisedTo`, `raisedToModulo`, `negated`, `shiftedLeft`, `shiftedRight` | copy form; truncation |
| `Decimal::divExact`, `divRound`, `pow` | `divideExact`, `divideRounded`, `power` | truncation |
| `Math::intDiv`, `fdiv`, `mod` | `quotient`, `divide`, `remainder` | truncation |
| `Arr::diff`, `Arr::intersect` | `difference`, `intersection` | truncation; symmetric pair |
| `Arr::countBy` | `groupCounts` | `count…` returns `uint` |
| `ObjectSet->intersect`, `diff` | `intersection`, `difference` | copy on a changing class; truncation |
| `Uri->resolve`, `sign` | `resolved`, `signed` | copy form |
| `DateTime->nanos()`, `TimeOfDay->nanos()`, the `nanos` option of `with` | `nanoseconds` | truncation |
| `Db\Connection->stream`, `streamAs` (and on `Db\Transaction`) | `queryEach`, `queryEachAs` | effect; matches `Json::decodeEach` |
| `Db\Connection->transaction` (and on `Db\Transaction`) | `runTransaction` | effect |
| `Db\Column->nullable()` | `isNullable()` | predicate |
| `Http\Client::stream` | `openStream` | effect; matches `openSocket` |
| `Ldap\Connection->compare` | `hasValue` | predicate; `compare…` returns `int` elsewhere |
| `Ldap\Connection->changes` | `searchChanges` | effect |
| `Http\TlsInfo->verified()`, `RateLimit\Decision->allowed()`, `WebPush\Sent->gone()` | `isVerified()`, `isAllowed()`, `isGone()` | predicate |
| `Test\Outcome->passed()` | `hasPassed()` | predicate |
| `Ldap\Ad\AccountFlags`: `script`, `disabled`, `homeDirectoryRequired`, `lockedOut`, `passwordNotRequired`, `reversibleEncryption`, `temporaryDuplicateAccount`, `normalAccount`, `interdomainTrustAccount`, `workstationTrustAccount`, `serverTrustAccount`, `passwordNeverExpires`, `mnsLogonAccount`, `smartcardRequired`, `trustedForDelegation`, `notDelegated`, `useDesKeyOnly`, `noPreauthRequired`, `passwordExpired`, `trustedToAuthForDelegation`, `partialSecretsAccount` | `hasLogonScript`, `isDisabled`, `isHomeDirectoryRequired`, `isLockedOut`, `isPasswordNotRequired`, `allowsReversibleEncryption`, `isTemporaryDuplicateAccount`, `isNormalAccount`, `isInterdomainTrustAccount`, `isWorkstationTrustAccount`, `isServerTrustAccount`, `hasNonExpiringPassword`, `isMnsLogonAccount`, `isSmartcardRequired`, `isTrustedForDelegation`, `isNotDelegated`, `usesDesKeyOnly`, `isPreauthNotRequired`, `isPasswordExpired`, `isTrustedToAuthenticateForDelegation`, `isPartialSecretsAccount` | predicate. `mustChangePassword` stays. The `with({…})` fields keep the flag names, but `trustedToAuthForDelegation` becomes `trustedToAuthenticateForDelegation` |
| `Ldap\Ad\GroupType`: `system`, `global`, `domainLocal`, `universal`, `appBasic`, `appQuery`, `security` | `isSystemGroup`, `isGlobal`, `isDomainLocal`, `isUniversal`, `isAppBasicGroup`, `isAppQueryGroup`, `isSecurityGroup` | predicate |
| `Novis\Image\Image->resize`, `crop`, `trim`, `rotate`, `flip`, `flatten`, `sharpen`, `blur`, `tint`, `grayscale` | `resized`, `cropped`, `trimmed`, `rotated`, `flipped`, `flattened`, `sharpened`, `blurred`, `tinted`, `grayscaled` | copy form |
| `Image->composite`, `brightness`, `contrast`, `gamma`, `text`, `format` | `withOverlay`, `withBrightness`, `withContrast`, `withGamma`, `withText`, `withFormat` | copy form; `format` means a text form elsewhere |
| `Image->metadata({ keep: true })` | `keepingMetadata()` | copy form; `{ keep: false }` was the default, so the bag goes |
| `Image->variants($set)` | `encodeVariants($set)` | effect: it encodes |
| `Novis\Pdf\Document->asset`, `standard`, `invoiceXml`, `password`, `created`, `header`, `footer`, `slots` | `withAsset`, `withStandard`, `withInvoiceXml`, `withPassword`, `withCreationTime`, `withHeader`, `withFooter`, `withSlots` | copy form; `Document` returns a new object |
| `Novis\Spreadsheet\Chart->series`, `combine` | `withSeries`, `combinedWith` | copy form; `Chart` returns a new object |
| `Novis\Spreadsheet\Sheet->add`, `addAll`, `merge` | `addRow`, `addRows`, `mergeCells` | in place; says what it adds |
| `Sheet->link`, `note`, `validate`, `group`, `table`, `conditional`, `image`, `chart` | `addLink`, `addNote`, `addValidation`, `addOutlineGroup`, `addTable`, `addConditionalFormat`, `addImage`, `addChart` | in place |
| `Sheet->print($options)` | `setPrintOptions($options)` | in place; `print` reads like "print now" |
| `Novis\Spreadsheet\Workbook->sheet`, `define` | `addSheet`, `defineName` | in place |

Unchanged on purpose: zero-argument reads (`Request::path()`), keyed reads (`Db\Row->int`,
`Request::header`), static factories (`Duration::seconds`, `Color::rgba`), `on…` handlers, the `Intl`
formatters, `Novis\Rest`, and every static member on a value (`Str::trim`, `Arr::sort`).

**Each slice renames its registry rows, its natives' symbols, its reference cards and its moved rows
together.** A symbol follows its member (`nvs_core_response_json` → `nvs_core_response_send_json`) so a
disassembly and a grep agree. Example directories move with their member (`docs/examples/core/Response/json/`
→ `sendJson/`), and so do benches (`benches/members/core/<Class>/<member>.nvs`) and every `// covers:`
directive. A moved directory keeps its files; `about.md` and comments are rewritten for the new name.

**What it spends.** Nothing on the request path: a name is resolved at compile time. Each moved row is
a few bytes in the binary until 1.0. `bun nv names --check` adds a few seconds to `ci --check`.
Usability: chains get a few characters longer, and every call says whether it reads, changes or copies.
Simplicity: one rule more, checked by a tool, against four meanings a reader had to look up.

**Declined.**
- **Parameter names** (`$a`, `$s`, `$fn`). A reader meets them at the declaration and in the reference,
  not at the call site this goal is about. `spawn`'s `args:` is the exception, because it is the same
  value as `Script::arguments()`.
- **A per-member effect flag in the registry.** Most of the rule follows from the types. The `reads`
  list closes the rest and fails on every new member of the one shape the types cannot judge.
- **Copy names for statics** (`Str::trimmed`). Nothing is changed in place, so there is nothing to mix
  them up with.
- **Keeping old names as aliases.** Pre-alpha has no transition period, and the moved row's fix rewrites
  each caller.
- **A naming check on a user's own program.** The rule is for `Core` and the first-party packages. A
  program names its own methods.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on. A rename that turns out to
break the rule another way is renamed again by the same rule, recorded under this section, and the
user is told in the handoff. A name not in the table that the check lists is renamed in stage 7 by the
same four parts, and recorded in this table.

**Lanes.** Stages 1 and 2 land first: 3 to 6 need the check, the moved pass and the sweep. Stages 3 to
6 change disjoint `crates/nvs-stdlib/src/` files but share callers across `docs/examples/`,
`tests/conformance/`, `website/` and the Rust tests, so they run in order, never as side goals. Inside a
stage the sweep's leftovers go to writing subagents by directory.

## Stages

### 1. the naming rules are rewritten, and bun nv names --check lists every name that breaks them

**Split:** none, one session. The check is expected to be red until stage 7.

- [ ] `docs/rules/core-api/verb-lexicon.md:1`: the four parts, one word one meaning, the first-party
  packages, and the Declined items above; `data/rules/core-api/verb-lexicon.json:9`'s `guardedBy`.
- [ ] `docs/rules/core-api/members-are-full-words.md:1`: the three lists by reference to
  `tools/data/api-names.json`, option fields and package class names included; its `guardedBy`.
- [ ] `docs/rules/core-api/shape-rules.md:1`: R5 and R7's cells name the new parts; `bun nv rules
  --render` once.
- [ ] `tools/data/api-names.json:1` and `tools/nv/cmd/names.ts:1`: the tables and the command, registered
  in `tools/nv/main.ts:56`; `--check` exits non-zero naming each member, the rule part and, for a
  truncation, the full word.
- [ ] `tools/nv/test/names.test.ts:1`: the seven tests the check `names-tool` names, on fixture
  signatures, run once.
- [ ] `tools/nv/cmd/ci.ts:332`: `api names` joins the list after the build, with `NVS_BIN` set.
- [ ] Run `bun nv names --check` once and paste its count by rule part into the handoff. Every name it
  lists outside the renames table is stage 7's.

### 2. a moved instance or package member gets its new name and a fix, and bun nv api-rename sweeps the tree

**Split:** none, one session.

- [ ] `crates/nvs-types/src/moved.rs:51`: the `E0405` subject is read for an instance call on a typed
  `Core` receiver and for a `Novis\…` package class; the fix writes the new member over the old one.
- [ ] `crates/nvs-stdlib/src/moved.rs:70`: the table's test accepts a package member row, or the names
  check validates those rows, as the standing decision says.
- [ ] `crates/nvs-syntax/src/parser/expr.rs:2687` and `crates/nvs-types/src/expr/quals.rs:2249`: spawn's
  `arguments:` key; `args:` is a compile error whose help names `arguments:`.
- [ ] The three tests the check `moved-members` names, run once by name.
- [ ] `tools/nv/cmd/api-rename.ts:1`: the sweep and `--check`, registered in `tools/nv/main.ts`;
  `tools/nv/test/api-rename.test.ts:1` with the four tests the check `api-rename-tool` names, run once.

### 3. the request path is renamed: Response, Request, Sse, Task, Metrics, Router\Match and Script

**Split:** the session renames the registry rows, symbols, cards and moved rows, then runs
`bun nv api-rename`. Its leftover list goes to writing subagents by directory: `docs/examples/`,
`tests/`, `website/` and `crates/*/tests/`. They run side by side, and the session builds once they end.

- [ ] `crates/nvs-stdlib/src/response.rs:376`, `:428`, `:474`: `sendJson`, `sendHtml`, `sendText`,
  `sendBytes`, `startStream`, `sendSlotted`.
- [ ] `crates/nvs-stdlib/src/request.rs:259`, `:379`: the six query and form reads.
- [ ] `crates/nvs-stdlib/src/sse.rs:181`, `:251`; `metrics.rs:190`; `task.rs:175`; `router.rs:441`,
  `:797`; `script.rs:165`.
- [ ] `crates/nvs-stdlib/src/moved.rs:70`: one row per rename above.
- [ ] `cargo build`, then `bun nv api-rename`; every leftover fixed; example and bench directories moved.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 4. the command line and the file system are renamed: Cli, Test, Xml\Writer, IO and Path

**Split:** as stage 3.

- [ ] `crates/nvs-stdlib/src/cli.rs:325`, `:335`, `:352`; `test.rs:825`.
- [ ] `crates/nvs-stdlib/src/xml.rs:777` to `:830`: the six `write…` members, and the cards that say
  "call `attribute`".
- [ ] `crates/nvs-stdlib/src/io.rs:195`, `:251`, `:294`, `:304`, `:347`, and `IO\Metadata->isDir`.
- [ ] `crates/nvs-stdlib/src/path.rs:109`, `:122`, `:214`, `:245`, and `rule:core-api/one-temporary-directory-member`'s name.
- [ ] The moved rows; `cargo build`, then `bun nv api-rename`; every leftover fixed; directories moved.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 5. values and data are renamed: BigInt, Decimal, Math, Arr, ObjectSet, Uri, Time, Db, Http and Ldap

**Split:** as stage 3. If the leftovers do not fit one session, the session stops after Db and Ldap
and the handoff names the rest.

- [ ] `crates/nvs-stdlib/src/bigint.rs:204` to `:314`; `decimal.rs:103`, `:113`, `:138`; `math.rs:189`,
  `:199`, `:209`. `BigInt->mod` is `remainder` only if its sign follows the dividend; the session reads
  the native first and records the answer in the card.
- [ ] `crates/nvs-stdlib/src/arr.rs:691` and `Arr::intersect`, `Arr::countBy`, the `Arr::SetOn` card;
  `objset.rs:130`, `:140`; `uri.rs:646`, `:666`.
- [ ] `crates/nvs-stdlib/src/time.rs:1247`: `nanoseconds` on `DateTime` and `TimeOfDay`, and the `with`
  option field.
- [ ] `crates/nvs-stdlib/src/db/registry.rs:509`, `:542`, `:700`, `:1666`, and the same on
  `Db\Transaction`; `http.rs:1785`, `:2124`; `ratelimit.rs:367`; `webpush.rs:207`;
  `test/hooks.rs:76`.
- [ ] `crates/nvs-stdlib/src/ldap/registry.rs:446`, `:536`; `ldap/flags.rs:167`, `:189`, `:290` and
  `ldap/registry.rs:2456`: the flag readers and the `with` field.
- [ ] The moved rows; `cargo build`, then `bun nv api-rename`; every leftover fixed; directories moved.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 6. the first-party packages are renamed: Novis\Image, Novis\Pdf and Novis\Spreadsheet

**Split:** one writing subagent per package for its `.nvs` source and tests, side by side; then the
session runs the sweep and hands the leftovers out as stage 3 does.

- [ ] `extensions/image/nvs/Image.nvs:212` and the rest of the `Image` table rows, `keepingMetadata`
  without a bag, and the image classification in `tools/data/api-names.json` (`value`).
- [ ] `extensions/pdf/nvs/Document.nvs:63` and the rest of the `Document` rows (`value`).
- [ ] `extensions/spreadsheet/nvs/Chart.nvs:84` (`value`), `Sheet.nvs:101` and `Workbook.nvs:1`
  (`changing`), and `Template` (`changing`).
- [ ] The moved rows for the package members; `cargo build`, then `bun nv api-rename`; every leftover
  fixed; directories moved.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 7. every further name the check lists is fixed, and no old name is left in the tree

**Split:** none, one session.

- [ ] `bun nv names --check`: each name it lists is renamed by the four parts, added to the renames table
  above, and given a moved row; a genuine read goes on the `reads` list.
- [ ] `bun nv names --check` and `bun nv api-rename --check` both exit 0.

### 8. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** writing subagents by directory for the `about.md` files and comments of the moved example
directories, held to AGENTS.md § *Text an end user reads*; the session writes the rules and the
changes entries.

- [ ] Every rule fragment under `docs/rules/` the sweep touched reads as the present, and
  `bun nv rules --check` is green.
- [ ] Each moved `about.md` and its examples' comments rewritten for the new name; each help card in the
  registry rows reread.
- [ ] `website/src/data/changes.json:1`: one `changed` entry per class family in the renames table,
  each naming the new features and, in one sentence, the old names. Each page the sweep touched reread
  and stamped with `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal clear-member-names` names nothing.

### 9. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 8 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the interactive session that settled the design with the user on 2026-11-24. Nothing is
built. The renames table was made from every signature in `website/src/data/core.json` that day, and
every anchor above was read that day.

### Next

Stage 1 from its first item, in `docs/rules/core-api/` and `tools/nv/`. Every check is red until its
stage's tests exist, and `names` stays red until stage 7.

### Backlog
