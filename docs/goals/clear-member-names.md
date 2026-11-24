---
looper: 1
title: "Every member that does something is named with a verb, and every API name says what it does"
summary: "Reached when the naming rules require a verb for every member with an effect, a copy-form name for every member that returns a changed copy, a predicate for every bool read and full words everywhere, bun nv names --check enforces them over Core and every first-party package, the moved-name table and the removed-option list are empty and their rules hold only after pre-alpha, so an old name is an unknown name and the changelog is its only record, every member that broke the rules is renamed, every caller in the tree uses the new names, and the rules, the reference, the examples, the feature proofs and the website say so."
context: [docs/rules/core-api/verb-lexicon.md, docs/rules/core-api/members-are-full-words.md, docs/rules/core-api/no-deprecation-window-before-a-release.md, docs/agent/changelog-summary.md]
stages:
  - id: 1
    title: "the naming rules are rewritten, and bun nv names --check lists every name that breaks them"
    status: done
    context: [docs/rules/core-api/shape-rules.md, docs/rules/core-api/symmetric-names.md, crates/nvs-stdlib/src/registry.rs, tools/nv/cmd/ci.ts, tools/nv/renderers/website-core.ts]
  - id: 2
    title: "the moved-name tables are empty until pre-alpha ends, and bun nv api-rename sweeps the tree from this goal's renames block"
    status: done
    context: [docs/rules/core-api/a-moved-name-says-where-it-went.md, crates/nvs-stdlib/src/moved.rs, crates/nvs-types/src/moved.rs, crates/nvs-types/src/attributes.rs, crates/nvs-syntax/src/parser/expr.rs]
  - id: 3
    title: "the request path is renamed: Response, Request, Sse, Task, Metrics, Router\\Match and Script"
    status: done
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/sse.rs]
  - id: 4
    title: "the command line and the file system are renamed: Cli, Test, Xml\\Writer, IO and Path"
    status: done
    context: [crates/nvs-stdlib/src/cli.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/path.rs]
  - id: 5
    title: "values and data are renamed: BigInt, Decimal, Math, Arr, ObjectSet, Uri, Time, Db, Http and Ldap"
    status: done
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
  - id: api-rename-tool
    stage: 2
    name: "bun nv api-rename reads a goal's renames block, rewrites qualified names, lists instance calls and checks for old names"
    run: [bun, test, tools/nv/test/api-rename.test.ts]
    tests: [the renames block of a goal file is read, a qualified static call is rewritten, a short name brought in by use is rewritten, an instance call is listed and not rewritten, check fails naming each old name left]
    timeout: 900
  - id: names
    stage: 7
    name: "every Core and first-party package name follows the naming rules"
    run: [bun, nv, names, --check]
    timeout: 900
  - id: no-old-names
    stage: 7
    name: "no name this goal renamed is written anywhere in the tree but this goal file and the changes entries"
    run: [bun, nv, api-rename, --check, --goal, clear-member-names]
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

The user's calls: do it properly, and breaking every caller is expected while Novis is pre-alpha.
**No list of old names is carried while Novis is pre-alpha.** Until then the changelog is the only
place a breaking change is mentioned. The rules that name a replacement
(`rule:core-api/a-moved-name-says-where-it-went` and the removed-option help) stay valid, but only from
the first release after pre-alpha: their mechanisms stay, and their tables are empty until then.
Everything else below is the session's design, built on unless the user picks otherwise.

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

**While pre-alpha, an old name is an unknown name, and the changelog is its only record.** Code that
writes a removed or renamed name gets the error any unknown name gets (`E0303`, `E0306`, `E0405`, or
the unknown-option error), with the ordinary closest-name suggestion where one exists. No row, help
line or fix names what replaced it. Stage 2:
- empties `MOVES` (`crates/nvs-stdlib/src/moved.rs:70`) and `REMOVED_OPTIONS`
  (`crates/nvs-types/src/attributes.rs:1292`). The types, the pass in `crates/nvs-types/src/moved.rs`,
  its callers (`crates/nvs-cli/src/main.rs:1540`, `crates/nvs-lsp/src/document.rs:1340`,
  `crates/nvs-lsp/src/actions.rs:289`) and the help branch stay, so the mechanism is ready for the
  first release after pre-alpha. An empty table costs one comparison per unknown-name error;
- keeps the mechanism tested with a `cfg(test)` table of made-up rows, and deletes the tests that pin
  real moves: `tests/conformance/lang/a-moved-core-member-says-where-it-went.nvst`,
  `tests/conformance/reject/a-removed-route-option-names-its-replacement.nvst`, the `MOVED` cases of
  `crates/nvs-cli/tests/check_flags.rs:377` to `:430`, and any other test the build names;
- rewrites `rule:core-api/a-moved-name-says-where-it-went` to start with its scope: it holds from the
  first release after pre-alpha, and until then both tables are empty and the changelog is the record.
  § *A row stays until 1.0* goes, and so does every sentence that cites a real pre-alpha move as its
  example (`docs/rules/core-classes/a-time-class-builds-its-own-values.md:88` among the citations);
- rewrites `rule:core-api/no-deprecation-window-before-a-release`: § *The error names the replacement*
  becomes "an old name gets the unknown-name error, and the changelog's Breaking changes section says
  what to write instead". Its Declined item "a bare unknown-name error with no replacement" becomes the
  rule for pre-alpha, with the user's reason: a list of old names is a surface to keep, and pre-alpha
  has no users to protect.

**The changelog entry is the rename's record.** Every commit that renames a member is
`feat(<scope>):` with a `BREAKING CHANGE:` body that lists each `Old` → `New`, so
[changelog-summary.md](../agent/changelog-summary.md) § *Breaking changes* picks it up with its
`Before:` example (its `git log --grep='BREAKING'` finds the body). The subject carries no `!`:
the synced `commit-msg` hook (`tools/looper/src/gates/commit-msg.ts:45`) refuses it, and the body is
what the changelog procedure reads. Stage 8 writes the matching `website/src/data/changes.json` entries.

**The sweep: `bun nv api-rename --goal <slug>`.** A command, because the same rewrite runs in four
stages. Its map is the `renames` block of this goal file (stage 2 writes it from the table below, one
fully qualified `Old New` pair per line), so the map is deleted with the goal and nothing in the tree
keeps it. The command:
- rewrites every `Class::old(` written in full, and every short name a `use` brings in, in every text
  file of the tree (`.nvs`, `.nvst`, `.lspt`, `.md`, `.rs` and `.ts` strings, website pages and snippets);
- lists each `->old(` call with `file:line`, because an instance call needs its receiver's type, for
  the session and its subagents to fix by reading. In a project, `nvs check` names the ones left as
  unknown members;
- with `--check`, exits non-zero and names every old name still written outside the goal file and
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
| `Script::args()` | `arguments()` | truncation; spawn's `with(args: …)` and `upgrade`'s `$args` become `arguments` with it, and `args:` is an unknown key |
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

**The `renames` block** is the table above, one fully qualified `Old New` pair per line, which
`bun nv api-rename --goal clear-member-names` reads. `Class::member` is a static member and
`Class->member` an instance member. An option field (`nanos`, `trustedToAuthForDelegation`, spawn's
`args:`, `Image->metadata`'s `keep`) is not a member, so it is not in the block: its slice renames it by
hand, and `bun nv names --check` names any left.

```renames
Core\Response::json Core\Response::sendJson
Core\Response::html Core\Response::sendHtml
Core\Response::text Core\Response::sendText
Core\Response::bytes Core\Response::sendBytes
Core\Response::slotted Core\Response::sendSlotted
Core\Response::stream Core\Response::startStream
Core\Request::query Core\Request::queryParameter
Core\Request::queryArray Core\Request::queryParameters
Core\Request::queryAs Core\Request::queryParametersAs
Core\Request::post Core\Request::formField
Core\Request::postArray Core\Request::formFields
Core\Request::postAs Core\Request::formFieldsAs
Core\Sse::stream Core\Sse::startStream
Core\Sse->retry Core\Sse->setRetryDelay
Core\Metrics::gauge Core\Metrics::setGauge
Core\Task::afterResponse Core\Task::runAfterResponse
Core\Router\Match->params Core\Router\Match->parameters
Core\Router\Match->param Core\Router\Match->parameter
Core\Script::args Core\Script::arguments
Core\Cli::secret Core\Cli::askSecret
Core\Cli::live Core\Cli::showLive
Core\Cli::progress Core\Cli::showProgress
Core\Test::scriptAnswers Core\Test::answerCli
Core\Xml\Writer->content Core\Xml\Writer->writeText
Core\Xml\Writer->attribute Core\Xml\Writer->writeAttribute
Core\Xml\Writer->comment Core\Xml\Writer->writeComment
Core\Xml\Writer->cdata Core\Xml\Writer->writeCdata
Core\Xml\Writer->instruction Core\Xml\Writer->writeInstruction
Core\Xml\Writer->doctype Core\Xml\Writer->writeDoctype
Core\IO::temporaryDir Core\IO::createTemporaryDirectory
Core\IO::makeDir Core\IO::makeDirectory
Core\IO::removeDir Core\IO::removeDirectory
Core\IO::isDir Core\IO::isDirectory
Core\IO::stat Core\IO::metadata
Core\IO\Metadata->isDir Core\IO\Metadata->isDirectory
Core\Path::dirname Core\Path::parent
Core\Path::basename Core\Path::fileName
Core\Path::fromCwd Core\Path::fromWorkingDirectory
Core\Path::thisDir Core\Path::thisDirectory
Core\BigInt->add Core\BigInt->plus
Core\BigInt->sub Core\BigInt->minus
Core\BigInt->mul Core\BigInt->multipliedBy
Core\BigInt->div Core\BigInt->dividedBy
Core\BigInt->mod Core\BigInt->remainder
Core\BigInt->pow Core\BigInt->raisedTo
Core\BigInt->powMod Core\BigInt->raisedToModulo
Core\BigInt->neg Core\BigInt->negated
Core\BigInt->shl Core\BigInt->shiftedLeft
Core\BigInt->shr Core\BigInt->shiftedRight
Core\Decimal::divExact Core\Decimal::divideExact
Core\Decimal::divRound Core\Decimal::divideRounded
Core\Decimal::pow Core\Decimal::power
Core\Math::intDiv Core\Math::quotient
Core\Math::fdiv Core\Math::divide
Core\Math::mod Core\Math::remainder
Core\Arr::diff Core\Arr::difference
Core\Arr::intersect Core\Arr::intersection
Core\Arr::countBy Core\Arr::groupCounts
Core\ObjectSet->intersect Core\ObjectSet->intersection
Core\ObjectSet->diff Core\ObjectSet->difference
Core\Uri->resolve Core\Uri->resolved
Core\Uri->sign Core\Uri->signed
Core\Time\DateTime->nanos Core\Time\DateTime->nanoseconds
Core\Time\TimeOfDay->nanos Core\Time\TimeOfDay->nanoseconds
Core\Db\Connection->stream Core\Db\Connection->queryEach
Core\Db\Connection->streamAs Core\Db\Connection->queryEachAs
Core\Db\Connection->transaction Core\Db\Connection->runTransaction
Core\Db\Transaction->stream Core\Db\Transaction->queryEach
Core\Db\Transaction->streamAs Core\Db\Transaction->queryEachAs
Core\Db\Transaction->transaction Core\Db\Transaction->runTransaction
Core\Db\Column->nullable Core\Db\Column->isNullable
Core\Http\Client::stream Core\Http\Client::openStream
Core\Ldap\Connection->compare Core\Ldap\Connection->hasValue
Core\Ldap\Connection->changes Core\Ldap\Connection->searchChanges
Core\Http\TlsInfo->verified Core\Http\TlsInfo->isVerified
Core\RateLimit\Decision->allowed Core\RateLimit\Decision->isAllowed
Core\WebPush\Sent->gone Core\WebPush\Sent->isGone
Core\Test\Outcome->passed Core\Test\Outcome->hasPassed
Core\Ldap\Ad\AccountFlags->script Core\Ldap\Ad\AccountFlags->hasLogonScript
Core\Ldap\Ad\AccountFlags->disabled Core\Ldap\Ad\AccountFlags->isDisabled
Core\Ldap\Ad\AccountFlags->homeDirectoryRequired Core\Ldap\Ad\AccountFlags->isHomeDirectoryRequired
Core\Ldap\Ad\AccountFlags->lockedOut Core\Ldap\Ad\AccountFlags->isLockedOut
Core\Ldap\Ad\AccountFlags->passwordNotRequired Core\Ldap\Ad\AccountFlags->isPasswordNotRequired
Core\Ldap\Ad\AccountFlags->reversibleEncryption Core\Ldap\Ad\AccountFlags->allowsReversibleEncryption
Core\Ldap\Ad\AccountFlags->temporaryDuplicateAccount Core\Ldap\Ad\AccountFlags->isTemporaryDuplicateAccount
Core\Ldap\Ad\AccountFlags->normalAccount Core\Ldap\Ad\AccountFlags->isNormalAccount
Core\Ldap\Ad\AccountFlags->interdomainTrustAccount Core\Ldap\Ad\AccountFlags->isInterdomainTrustAccount
Core\Ldap\Ad\AccountFlags->workstationTrustAccount Core\Ldap\Ad\AccountFlags->isWorkstationTrustAccount
Core\Ldap\Ad\AccountFlags->serverTrustAccount Core\Ldap\Ad\AccountFlags->isServerTrustAccount
Core\Ldap\Ad\AccountFlags->passwordNeverExpires Core\Ldap\Ad\AccountFlags->hasNonExpiringPassword
Core\Ldap\Ad\AccountFlags->mnsLogonAccount Core\Ldap\Ad\AccountFlags->isMnsLogonAccount
Core\Ldap\Ad\AccountFlags->smartcardRequired Core\Ldap\Ad\AccountFlags->isSmartcardRequired
Core\Ldap\Ad\AccountFlags->trustedForDelegation Core\Ldap\Ad\AccountFlags->isTrustedForDelegation
Core\Ldap\Ad\AccountFlags->notDelegated Core\Ldap\Ad\AccountFlags->isNotDelegated
Core\Ldap\Ad\AccountFlags->useDesKeyOnly Core\Ldap\Ad\AccountFlags->usesDesKeyOnly
Core\Ldap\Ad\AccountFlags->noPreauthRequired Core\Ldap\Ad\AccountFlags->isPreauthNotRequired
Core\Ldap\Ad\AccountFlags->passwordExpired Core\Ldap\Ad\AccountFlags->isPasswordExpired
Core\Ldap\Ad\AccountFlags->trustedToAuthForDelegation Core\Ldap\Ad\AccountFlags->isTrustedToAuthenticateForDelegation
Core\Ldap\Ad\AccountFlags->partialSecretsAccount Core\Ldap\Ad\AccountFlags->isPartialSecretsAccount
Core\Ldap\Ad\GroupType->system Core\Ldap\Ad\GroupType->isSystemGroup
Core\Ldap\Ad\GroupType->global Core\Ldap\Ad\GroupType->isGlobal
Core\Ldap\Ad\GroupType->domainLocal Core\Ldap\Ad\GroupType->isDomainLocal
Core\Ldap\Ad\GroupType->universal Core\Ldap\Ad\GroupType->isUniversal
Core\Ldap\Ad\GroupType->appBasic Core\Ldap\Ad\GroupType->isAppBasicGroup
Core\Ldap\Ad\GroupType->appQuery Core\Ldap\Ad\GroupType->isAppQueryGroup
Core\Ldap\Ad\GroupType->security Core\Ldap\Ad\GroupType->isSecurityGroup
Novis\Image\Image->resize Novis\Image\Image->resized
Novis\Image\Image->crop Novis\Image\Image->cropped
Novis\Image\Image->trim Novis\Image\Image->trimmed
Novis\Image\Image->rotate Novis\Image\Image->rotated
Novis\Image\Image->flip Novis\Image\Image->flipped
Novis\Image\Image->flatten Novis\Image\Image->flattened
Novis\Image\Image->sharpen Novis\Image\Image->sharpened
Novis\Image\Image->blur Novis\Image\Image->blurred
Novis\Image\Image->tint Novis\Image\Image->tinted
Novis\Image\Image->grayscale Novis\Image\Image->grayscaled
Novis\Image\Image->composite Novis\Image\Image->withOverlay
Novis\Image\Image->brightness Novis\Image\Image->withBrightness
Novis\Image\Image->contrast Novis\Image\Image->withContrast
Novis\Image\Image->gamma Novis\Image\Image->withGamma
Novis\Image\Image->text Novis\Image\Image->withText
Novis\Image\Image->format Novis\Image\Image->withFormat
Novis\Image\Image->metadata Novis\Image\Image->keepingMetadata
Novis\Image\Image->variants Novis\Image\Image->encodeVariants
Novis\Pdf\Document->asset Novis\Pdf\Document->withAsset
Novis\Pdf\Document->standard Novis\Pdf\Document->withStandard
Novis\Pdf\Document->invoiceXml Novis\Pdf\Document->withInvoiceXml
Novis\Pdf\Document->password Novis\Pdf\Document->withPassword
Novis\Pdf\Document->created Novis\Pdf\Document->withCreationTime
Novis\Pdf\Document->header Novis\Pdf\Document->withHeader
Novis\Pdf\Document->footer Novis\Pdf\Document->withFooter
Novis\Pdf\Document->slots Novis\Pdf\Document->withSlots
Novis\Spreadsheet\Chart->series Novis\Spreadsheet\Chart->withSeries
Novis\Spreadsheet\Chart->combine Novis\Spreadsheet\Chart->combinedWith
Novis\Spreadsheet\Sheet->add Novis\Spreadsheet\Sheet->addRow
Novis\Spreadsheet\Sheet->addAll Novis\Spreadsheet\Sheet->addRows
Novis\Spreadsheet\Sheet->merge Novis\Spreadsheet\Sheet->mergeCells
Novis\Spreadsheet\Sheet->link Novis\Spreadsheet\Sheet->addLink
Novis\Spreadsheet\Sheet->note Novis\Spreadsheet\Sheet->addNote
Novis\Spreadsheet\Sheet->validate Novis\Spreadsheet\Sheet->addValidation
Novis\Spreadsheet\Sheet->group Novis\Spreadsheet\Sheet->addOutlineGroup
Novis\Spreadsheet\Sheet->table Novis\Spreadsheet\Sheet->addTable
Novis\Spreadsheet\Sheet->conditional Novis\Spreadsheet\Sheet->addConditionalFormat
Novis\Spreadsheet\Sheet->image Novis\Spreadsheet\Sheet->addImage
Novis\Spreadsheet\Sheet->chart Novis\Spreadsheet\Sheet->addChart
Novis\Spreadsheet\Sheet->print Novis\Spreadsheet\Sheet->setPrintOptions
Novis\Spreadsheet\Workbook->sheet Novis\Spreadsheet\Workbook->addSheet
Novis\Spreadsheet\Workbook->define Novis\Spreadsheet\Workbook->defineName
```

Unchanged on purpose: zero-argument reads (`Request::path()`), keyed reads (`Db\Row->int`,
`Request::header`), static factories (`Duration::seconds`, `Color::rgba`), `on…` handlers, the `Intl`
formatters, `Novis\Rest`, and every static member on a value (`Str::trim`, `Arr::sort`).

**Each slice renames its registry rows, its natives' symbols and its reference cards together.** A symbol follows its member (`nvs_core_response_json` → `nvs_core_response_send_json`) so a
disassembly and a grep agree. Example directories move with their member (`docs/examples/core/Response/json/`
→ `sendJson/`), and so do benches (`benches/members/core/<Class>/<member>.nvs`) and every `// covers:`
directive. A moved directory keeps its files; `about.md` and comments are rewritten for the new name.

**What it spends.** Nothing on the request path: a name is resolved at compile time. The moved table
loses its rows. `bun nv names --check` adds a few seconds to `ci --check`. Usability:
chains get a few characters longer, and every call says whether it reads, changes or copies. A program
written against an old name gets a plain unknown-name error, and its author reads the changelog.
Simplicity: one rule more, checked by a tool, against four meanings a reader had to look up.

**Declined.**
- **Parameter names** (`$a`, `$s`, `$fn`). A reader meets them at the declaration and in the reference,
  not at the call site this goal is about. `spawn`'s `args:` is the exception, because it is the same
  value as `Script::arguments()`.
- **A per-member effect flag in the registry.** Most of the rule follows from the types. The `reads`
  list closes the rest and fails on every new member of the one shape the types cannot judge.
- **Copy names for statics** (`Str::trimmed`). Nothing is changed in place, so there is nothing to mix
  them up with.
- **Keeping old names as aliases, or rows that name where each went, while pre-alpha.** The user's
  call: until pre-alpha ends the changelog is the only record of a breaking change.
- **Deleting the moved-name mechanism.** Its rules are valid after pre-alpha, so it stays with empty
  tables instead of being written again later.
- **A naming check on a user's own program.** The rule is for `Core` and the first-party packages. A
  program names its own methods.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on. A rename that turns out to
break the rule another way is renamed again by the same rule, recorded under this section, and the
user is told in the handoff. A name not in the table that the check lists is renamed in stage 7 by the
same four parts, and recorded in this table and the `renames` block.

**What stage 1 settled in the check's tables** (`tools/data/api-names.json`), each by the four parts:
- `verify…`, `check…` and `assert…` are effect verbs: a check that throws when it fails stops the
  program, so `Crypto::verify(): void` and `Password::verify(): bool` keep their names.
- A participle ending in `en` counts only from `irregularParticiples`, since `flatten`, `sharpen` and
  `open` end so too. A word ending in `ed` counts unless it is on `notParticiples` (`embed`).
- `approx` joins the banned truncations (`approximately`), so `LdapFilter::approx` is stage 7's.
- Drawing from a generator is a read: `Random::int`, `RandomSeeded->int`, `->bytes` and `->token`
  are on `reads`. `NovisRest` is unchanged, so its nouns with arguments are on `reads` too.
- `DateTime->next`, `previous`, `startOf`, `endOf`, `Dn->child` and `ObjectSet->union` compute a new
  value rather than a changed copy, so they are `reads` rows and keep their names. So do the new names
  the table gives a noun with an argument (`Arr::difference`, `Math::remainder`, `BigInt->remainder`,
  `Decimal::power`, `RouterMatch->parameter`, `ObjectSet->intersection`, `difference`).
- `query` is an effect verb (`DbConnection->query` runs a statement), which is why
  `Request::query` moves.
- The check labels part 2 for "a noun with an argument that is not on `reads`", since part 2 is the
  rule that a noun has no effect.

**What stage 3 settled for the sweep's neighbours.**
- **A ledger row follows its member.** `data/perf/members.md` and `bench-budget.json` are the present
  state of each bench, keyed by its path or member, so their rows are renamed. The append-only logs
  (`data/perf/members.ndjson`) and `data/proofs/released.json` (what `v0.0.5` shipped) keep the old
  names, because they are history.
- **A feature id is `Class::member` even for an instance member** (`Core\Sse::retry`), so the sweep's
  `->` pattern misses it. Each stage renames those ids by hand in the `.json` data (`questions.json`,
  `site.lock.json`, `review-baseline.json`, `intro-backlog.json`, `data/rules/**`), in `// covers:`
  lines and in messages. `website/src/data/core.json` and `website/public/llms-full.txt` are generated,
  and stage 8 regenerates them.
- **A sweep is run with `--class` for the stage's classes**, and its own examples are made up
  (`Shop\Reply::json`), so a sweep never rewrites the tool's text.
- **A rule id and a file name that hold an old word keep it until stage 8:** `rule:core-classes/script-args`,
  the `script-args-*.nvst` cases, `05-give-the-child-a-value-with-args.nvs`. A rule id is cited across
  the tree, so one pass renames them all.

**Lanes.** Stages 1 and 2 land first: 3 to 6 need the check and the sweep. Stages 3 to
6 change disjoint `crates/nvs-stdlib/src/` files but share callers across `docs/examples/`,
`tests/conformance/`, `website/` and the Rust tests, so they run in order, never as side goals. Inside a
stage the sweep's leftovers go to writing subagents by directory.

## Stages

### 1. the naming rules are rewritten, and bun nv names --check lists every name that breaks them

**Split:** none, one session. The check is expected to be red until stage 7.

- [x] `docs/rules/core-api/verb-lexicon.md:1`: the four parts, one word one meaning, the first-party
  packages, and the Declined items above; `data/rules/core-api/verb-lexicon.json:9`'s `guardedBy`.
- [x] `docs/rules/core-api/members-are-full-words.md:1`: the three lists by reference to
  `tools/data/api-names.json`, option fields and package class names included; its `guardedBy`.
- [x] `docs/rules/core-api/shape-rules.md:1`: R5 and R7's cells name the new parts; `bun nv rules
  --render` once.
- [x] `tools/data/api-names.json:1` and `tools/nv/cmd/names.ts:1`: the tables and the command, registered
  in `tools/nv/main.ts:56`; `--check` exits non-zero naming each member, the rule part and, for a
  truncation, the full word.
- [x] `tools/nv/test/names.test.ts:1`: the seven tests the check `names-tool` names, on fixture
  signatures, run once.
- [x] `tools/nv/cmd/ci.ts:332`: `api names` joins the list after the build, with `NVS_BIN` set.
- [x] Run `bun nv names --check` once and paste its count by rule part into the handoff. Every name it
  lists outside the renames table is stage 7's.

### 2. the moved-name tables are empty until pre-alpha ends, and bun nv api-rename sweeps the tree from this goal's renames block

**Split:** none, one session.

- [x] `crates/nvs-stdlib/src/moved.rs:70` and `crates/nvs-types/src/attributes.rs:1292`: `MOVES` and
  `REMOVED_OPTIONS` empty; a `cfg(test)` table of made-up rows keeps the pass and the help branch
  tested; the tests pinning real moves deleted, as the standing decision lists them.
- [x] `docs/rules/core-api/a-moved-name-says-where-it-went.md:1` and
  `docs/rules/core-api/no-deprecation-window-before-a-release.md:1` rewritten as the standing decision
  says; every citation of a real pre-alpha move rewritten; `bun nv rules --check` green.
- [x] `crates/nvs-syntax/src/parser/expr.rs:2687` and `crates/nvs-types/src/expr/quals.rs:2249`: spawn's
  `arguments:` key replaces `args:`, which is an unknown key.
- [x] The `renames` block under § *Standing decisions*, written from the renames table: one fully
  qualified `Old New` pair per line.
- [x] `tools/nv/cmd/api-rename.ts:1`: the sweep and `--check`, both taking `--goal <slug>`, registered in
  `tools/nv/main.ts`; `tools/nv/test/api-rename.test.ts:1` with the five tests the check
  `api-rename-tool` names, run once.

### 3. the request path is renamed: Response, Request, Sse, Task, Metrics, Router\Match and Script

**Split:** the session renames the registry rows, symbols and cards, then runs
`bun nv api-rename --goal clear-member-names`. Its list of instance calls goes to writing subagents by
directory: `docs/examples/`, `tests/`, `website/` and `crates/*/tests/`. They run side by side, and the
session builds once they end. Each stage commits as `feat(<scope>):` with a `BREAKING CHANGE:` body
listing every `Old` → `New` it renamed, as the standing decision says.

- [x] `crates/nvs-stdlib/src/response.rs:376`, `:428`, `:474`: `sendJson`, `sendHtml`, `sendText`,
  `sendBytes`, `startStream`, `sendSlotted`.
- [x] `crates/nvs-stdlib/src/request.rs:259`, `:379`: the six query and form reads.
- [x] `crates/nvs-stdlib/src/sse.rs:181`, `:251`; `metrics.rs:190`; `task.rs:175`; `router.rs:441`,
  `:797`; `script.rs:165`.
- [x] `Core\Socket::upgrade` and `Core\Sse::upgrade`: the `$args` parameter becomes `$arguments`, with
  every `args:` named argument (`apps/chat/src/App.nvs:87`, `apps/poll/src/App.nvs:98`, the upgrade
  benches, examples, hostile tests and conformance cases). Then every prose `args:` that means spawn's
  or upgrade's option says `arguments:`: `docs/reference/lang/80-concurrency.md:1026`, the
  `isolates-spawn-script-and-await` example and its `details.md`, `rule:security/isolate-shares-nothing`,
  the Rust doc comments `grep -rn '\`args:\`' crates` lists, and `crates/nvs-stdlib/src/socket.rs:832`,
  `:1029` and `sse.rs:620`'s messages. Spawn's key and the runtime's mismatch message already say
  `arguments:`.
- [x] `cargo build`, then `bun nv api-rename --goal clear-member-names`; every listed call fixed; example
  and bench directories moved.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 4. the command line and the file system are renamed: Cli, Test, Xml\Writer, IO and Path

**Split:** as stage 3.

- [x] `crates/nvs-stdlib/src/cli.rs:325`, `:335`, `:352`; `test.rs:825`.
- [x] `crates/nvs-stdlib/src/xml.rs:777` to `:830`: the six `write…` members, and the cards that say
  "call `attribute`".
- [x] `crates/nvs-stdlib/src/io.rs:195`, `:251`, `:294`, `:304`, `:347`, and `IO\Metadata->isDir`.
- [x] `crates/nvs-stdlib/src/path.rs:109`, `:122`, `:214`, `:245`, and `rule:core-api/one-temporary-directory-member`'s name.
- [x] `cargo build`, then `bun nv api-rename --goal clear-member-names`; every listed call fixed;
  directories moved.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 5. values and data are renamed: BigInt, Decimal, Math, Arr, ObjectSet, Uri, Time, Db, Http and Ldap

**Split:** as stage 3. If the leftovers do not fit one session, the session stops after Db and Ldap
and the handoff names the rest.

- [x] `crates/nvs-stdlib/src/bigint.rs:204` to `:314`; `decimal.rs:103`, `:113`, `:138`; `math.rs:189`,
  `:199`, `:209`. `BigInt->mod` is `remainder` only if its sign follows the dividend; the session reads
  the native first and records the answer in the card.
- [x] `crates/nvs-stdlib/src/arr.rs:691` and `Arr::intersect`, `Arr::countBy`, the `Arr::SetOn` card;
  `objset.rs:130`, `:140`; `uri.rs:646`, `:666`.
- [x] `crates/nvs-stdlib/src/time.rs:1247`: `nanoseconds` on `DateTime` and `TimeOfDay`, and the `with`
  option field.
- [x] `crates/nvs-stdlib/src/db/registry.rs:509`, `:542`, `:700`, `:1666`, and the same on
  `Db\Transaction`; `http.rs:1785`, `:2124`; `ratelimit.rs:367`; `webpush.rs:207`;
  `test/hooks.rs:76`.
- [x] `crates/nvs-stdlib/src/ldap/registry.rs:446`, `:536`; `ldap/flags.rs:167`, `:189`, `:290` and
  `ldap/registry.rs:2456`: the flag readers and the `with` field.
- [x] `cargo build`, then `bun nv api-rename --goal clear-member-names`; every listed call fixed;
  directories moved.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 6. the first-party packages are renamed: Novis\Image, Novis\Pdf and Novis\Spreadsheet

**Split:** one writing subagent per package for its `.nvs` source and tests, side by side; then the
session runs the sweep and hands the leftovers out as stage 3 does.

- [ ] `extensions/image/nvs/Image.nvs:212` and the rest of the `Image` table rows, `keepingMetadata`
  without a bag, and the image classification in `tools/data/api-names.json` (`value`).
- [ ] `extensions/pdf/nvs/Document.nvs:63` and the rest of the `Document` rows (`value`).
- [ ] `extensions/spreadsheet/nvs/Chart.nvs:84` (`value`), `Sheet.nvs:101` and `Workbook.nvs:1`
  (`changing`), and `Template` (`changing`).
- [ ] `cargo build`, then `bun nv api-rename --goal clear-member-names`; every listed call fixed;
  directories moved.
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` green.

### 7. every further name the check lists is fixed, and no old name is left in the tree

**Split:** none, one session.

- [ ] `bun nv names --check`: each name it lists is renamed by the four parts, added to the renames table
  and the `renames` block above; a genuine read goes on the `reads` list.
- [ ] `bun nv names --check` and `bun nv api-rename --check --goal clear-member-names` both exit 0.

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

Stages 1 to 5 are done. Stage 5 renamed every stage 5 row of the table: rows, symbols, cards, labels,
directories, feature ids, callers, and the bare-string tables in `registry.rs`,
`nvs-types/src/intrinsics.rs` and `nvs-ir/src/forms.rs`. `BigInt->remainder` takes the dividend's
sign. The `nanos` option is `nanoseconds` on all four members, `Instant::fromEpoch` and
`TimeOfDay::at` included; the slot stays `nanos`, and so does the intl extension's WIT field
(`extensions/intl/nvs/DateFormat.nvs:12`). `questions.json`'s keys match stages 3 to 5. Build,
clippy, `cargo fmt`, `rules --check` and the filtered `api-rename --check` are green; no test ran.

Expect red at stage 9 if anything was missed: `.lspt` completion cases and `nvs-test`'s
`TOP_LEVEL_ONLY` cases. `echo` labels such as `powMod` kept their `--EXPECT--` lines. `core.json`,
`core-changelog.json`, `llms-full.txt` and `site.lock.json`'s stamps are stale until stage 8.

Red until stage 7: `bun nv names --check`, the unfiltered `api-rename --check`, so `ci --check`, the
floor's `ci-check` and `ci --push`. Stage 7's names outside the table: `CoreProcess::run`, `spawn`'s
`cwd`, `Queue::push`'s `args`, `NovisImageDiff`, `RateLimit::shed`, `Cli::multiSelect` and
`LdapFilter::approx`.

### Next

Stage 6, the packages, from its first item: `extensions/image/nvs/Image.nvs:212`,
`extensions/pdf/nvs/Document.nvs:63`, `extensions/spreadsheet/nvs/Chart.nvs:84`, `Sheet.nvs:101` and
`Workbook.nvs:1`. The members are declared in Novis, so there are no Rust symbols. Check
`extensions/*/manifest.json` and each extension's Rust tests for bare names.
1. Rename the declarations in the `.nvs` files. `Image->metadata({ keep: true })` becomes
   `keepingMetadata()` with no bag.
2. Run `bun nv api-rename --goal clear-member-names --class 'Novis\Image\Image' --class …` into
   `.agent-tmp/`, then grep for `->old<` beside `->old(`. The sweep lists only `->old(`. Fix the list
   by line, after excluding the receivers of another class. `->add(`, `->text(`, `->format(`,
   `->merge(` and `->print(` have many.
3. Move the directories under `docs/examples/novis/`, `tests/hostile/novis/` and
   `benches/members/novis/`. Then rename the `Class::member` feature ids in every file, `.json` and
   `.toml` included. `.agent-tmp/stage5_map.py`, `stage5_move.py` and `stage5_ids.py` do this; edit
   their map and their `core/` prefix to `novis/`.
4. Give the bare mentions to three writing subagents, as stage 5 did: `.rs` files; the example,
   hostile, bench and conformance trees; and `docs/rules`, `docs/reference`, `data/` and the website.
5. Run `bun nv rules --render`, the filtered `--check`, `cargo fmt`, build and clippy.

### Backlog

- `bun nv api-rename` lists `->old(` but not `->old<T>(`. Stage 7 adds the generic form to its
  listing and to `--check` (`tools/nv/cmd/api-rename.ts`).
- `crates/nvs-stdlib/src/test.rs:481`: the `Test::tlsSession` option `verified` sets what
  `TlsInfo->isVerified()` returns. Stage 7 decides whether it becomes `isVerified` by part 4.
- Stage 3 prose left behind: `docs/examples/core/Response/startStream/01-…nvs:5` and its `about.md`
  (lines 1, 7, 8) still say `stream`. `docs/examples/core/Sse/startStream/01-…nvs:4` and its
  `about.md` (lines 6, 10) still say `stream()`. Stage 8 rewrites them.
- `data/reference/core/Math.json:12` keeps `fdiv`, `intdiv` and `pow` as search keywords, because
  they are PHP's names. Stage 8 decides whether `quotient` and `divide` join them.
- `rule:core-classes/script-args`, the `script-args-*.nvst` cases and
  `05-give-the-child-a-value-with-args.nvs` still say `args`. Stage 8 renames them together.
- File names and rule ids that still hold an old word, for stage 8's one pass. From stage 4:
  `rule:core-classes/temporary-dir-sweep`; `tests/conformance/core/test-script-answers-*.nvst` (3),
  `cli-secret-originates-…`, `io-stat-agrees-…`, `io-a-stat-is-a-snapshot-…`,
  `io-is-file-and-is-dir-partition-…`, `io-make-dir-*.nvst` (3), `io-temporary-dir-is-refused-…`,
  `path-dirname-and-basename-…` (named at `path-normalize-has-a-normal-form-…nvst:71`),
  `path-from-cwd-…`, `path-this-dir-…`, `path-this-file-and-this-dir-…`,
  `reject/path-a-tainted-path-not-from-cwd-…`; the Rust test `…_and_script_answers_write_the_ledger…`
  at `crates/nvs-stdlib/src/test.rs:8802`, and the helpers `fn basename`/`fn dirname` in
  `crates/nvs-stdlib/src/path.rs:1318`. From stage 5: the conformance cases named
  `bigint-div-and-mod-…`, `decimal-div-round-…`, `db-stream-*`, `db-transaction-stream-*`,
  `ldap-compare-*`, `ldap-changes-*`, `uri-sign-*`, `uri-resolve-*`, `uri-with-and-resolve`,
  `time-from-epoch-…-its-nanos-…`, and the playbook file
  `core-objectsets-union-intersect-and-diff-…md`.
- `bun nv api-rename`'s `TEXT_EXTENSIONS` lacks `.json`, `.toml` and `.out`. Stage 7 adds them, so
  its `--check` sees the data files (the generated `core.json`, `llms-full.txt` and the history files
  stay excluded).
- `crates/nvs-stdlib/src/request.rs:1872` says "which used to have its own", which is history in a
  comment. Rewrite it when the file is next open.
- `docs/rules/config/an-entry-runs-on-a-server-event.md:79` says "the args map". Check whether it means
  `Queue::push`'s `args`, which stage 7 renames.
