# Changelog summary procedure

What a session does when the person asks for a changelog summary of the raw changes. The ask comes in
words such as "write me the changelog summary" or "summarize what changed since v0.0.4". The result is
a short article for people who use Novis, written above the commit list that `bun nv release --notes`
renders. It names every change a user can see, in plain words, with a short example for every change
to what a user writes.

**This file is only the procedure.** The wording is held to AGENTS.md § *Text an end user reads* and
[prose.md](prose.md), and nothing here restates them. The commit list itself is
`tools/nv/cmd/release.ts`'s, and this procedure never changes it.

**A person asks for it, always.** The procedure writes nothing into the tree and publishes nothing. The
summary goes into the reply. It goes into a GitHub release, `CHANGELOG.md` or anywhere else only when
the person asks for that, each time.

## 1. The range

The default range is the last release tag to `HEAD`: `git describe --tags --abbrev=0` gives the tag.
A version or range the person names wins. The reply states the range and its commit count.

## 2. Collect the candidates

Read the subjects of the range with `git log --reverse --format='%h%x09%s' <range>`, and the bodies of
the ones that carry `BREAKING CHANGE:` with `git log --grep='BREAKING' <range>`.

Three sources say what a user sees, in this order of trust:

1. **`website/src/data/changes.json`, where it exists.** Each entry is already written for users. It is
   used as written, and a commit that it covers adds nothing.
2. **The goals the range retired.** A `chore(goals): <slug> is reached and retired` commit marks one.
   The goal's theme in `website/src/data/roadmap.json`, read at the commit before the one that drops
   it, is already user text: `git show <commit>^:website/src/data/roadmap.json`. These themes feed the
   opening paragraph.
3. **The commits.** Their subjects are written for contributors. Each one is a pointer to a change,
   never text to copy.

Each commit goes to the section of **what a user sees**, not to the section its type names:

| A user sees | Section | Commits that usually land there |
|---|---|---|
| a vulnerability closed | Security fixes | type or scope `security` only |
| code or a setting that must change | Breaking changes | `!` after the type, or `BREAKING CHANGE:` in the body |
| something they can now write or run | New | `feat` on a `Core` class, the language, a setting, a command, the editor |
| the same program, faster or smaller | Faster | `perf`, and a `feat` in the compiler that changes no result |
| a wrong result, a crash or an error that is gone | Fixes | `fix` |
| nothing | Internal changes | `refactor`, `build`, `test`, `chore`, `docs`, and any type scoped `nv`, `test`, `site`, `ci`, `looper`, `goals`, `docs`, `bench`, `release`, `roadmap` or `proofs` |

- **A security fix is never inferred.** A subject that says `attack`, `taint` or `escape` is not one.
  With no `security` commit and no `security: true` entry, the section is left out.
- **A hidden flag is not something a user writes.** `--no-in-place` and `--no-cheaper-forms` exist for
  tests, so a commit that adds one is internal.
- **A range of more than about 150 commits is read by subagents** (AGENTS.md § *The context budget*).
  Split the candidates by section, one `reader` subagent each. Each returns, per item: whether a user
  sees it, the exact name and signature, one line on what changed, and the `file:line` that shows it.
  The session checks each pointer before it uses it.

## 3. Verify each item before writing it

A subject is never enough to write an item from.

- **Read the change, then the feature's own docs.** Read the commit's diff first. Then read the `Core`
  reference card under `docs/reference/core/`, or the feature's `about.md` under `docs/examples/`. For a
  command, read `nvs help <command>` or its chapter under `docs/reference/tools/`. The name, the
  parameters and their order come from those, never from the subject.
- **An item whose effect cannot be stated from that evidence is not written.** It moves to Internal
  changes, and the reply lists it under *Not verified*.
- **No figure is written that the tree does not record.** "Faster" needs no number. A number, when
  given, names what it measures and where the tree records it.

## 4. The shape

```markdown
## Novis <version, or the range>

<The opening: two to four sentences on the changes that matter most to a user.>

### Security fixes
### Breaking changes
### New
### Faster
### Fixes
### Internal changes
```

- **The sections keep this order, and an empty one is left out.**
- **The opening names two or three changes.** It says what a user can do now. It does not list the
  section titles, and it does not greet.
- **One bullet per change a user sees.** Commits that make one change are one bullet. A bullet is one
  or two sentences: what changed, then what the result is.
- **At most eight bullets in a section.** Keep the eight a user meets most. One sentence after them
  says that more are in the commit list.
- **Security fixes** say what an attacker could do before, that it is fixed, and that users should
  update.
- **Breaking changes** say what changed and what to write instead.
- **Faster** says which kind of program or which function is faster.
- **Internal changes** is one paragraph of at most three sentences, never a list.
- **No internals in the text:** no goal slug, commit hash, rule id, crate name, Rust type or commit
  scope (`ir:`, `stdlib:`). No word from the commit subjects that a user would not know, such as
  "lowering", "ownership pass" or "a sole subject".

## 5. A code example for every change to what a user writes

A bullet in Breaking changes or New, or in Fixes when the fix changes what a call returns, gets a short
example right under it. That covers a new or changed `Core` function, method, class or parameter. It
also covers a syntax form, an attribute, an `nvs.toml` setting and a `package.toml` table. A command or
flag gets one when `nvs help` shows it.

- **The example is short.** At most ten lines, and only what the change needs.
- **The fence names the language:** `nvs` for a program, `toml` for a setting, `sh` for a command.
- **A program starts with `<?nvs`** and runs as it is.
- **A comment in the example follows AGENTS.md § *Text an end user reads*.** It says what the line does,
  then the result, with the real value: "This prints `6`."
- **A breaking change shows the new code.** The old line stays in it as a comment marked `Before:`.

A `New` bullet with its example:

````markdown
- A `[[restrict]]` table in `package.toml` says where a class or method may be used. A use outside the
  allowed places is error `E0664` when the program compiles.

  ```toml
  # Only code in `Framework\Core\Signing` and below may call `Core\Router::urlSigned`.
  [[restrict]]
  use   = 'Core\Router::urlSigned'
  allow = ['Framework\Core\Signing\*']
  note  = "Use `Framework\\Core\\Signing\\Url::signed`."
  ```
````

A `Faster` bullet, first as a commit subject reads and then as a user reads it:

```markdown
- `padEnd`, `padStart` and `repeat` write into a sole subject's own allocation.
- `Core\Str::padStart`, `padEnd` and `repeat` are faster on a string that no other variable uses.
```

## 6. Check it before handing it over

Write the summary and each example under `.agent-tmp/changelog/`.

1. **Every program runs.** Build the range's end with `cargo build`, run each example with
   `target/debug/nvs run <file>`, and compare what it prints with what its comment says. Run a command
   example as it is written. Put a setting into an `nvs.toml` beside a one-line program, and run that.
2. **Every name in backticks exists** at the range's end: in a reference card, an `about.md` or the
   output of `nvs help`.
3. **`bun nv prose .agent-tmp/changelog/summary.md` passes.** A line it names is written again from what
   the change does. It is never cut down until the check passes.
4. **Read it once as a newcomer.** Each bullet says what the reader can now do, or what they must
   change. A bullet that only makes sense to somebody who works on Novis is written again.
5. **Delete `.agent-tmp/changelog/`** once the summary is in the reply.

## 7. Hand it over

The reply carries the summary as Markdown and, under it:

- the range and its commit count, and the number of bullets in each section;
- *Not verified*: each item whose effect could not be shown, with its commit;
- where the summary goes next is the person's call: the GitHub draft release, above its commit list,
  or nowhere.
