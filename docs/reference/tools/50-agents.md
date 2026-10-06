---
id: agents
title: "Coding agents: nvs agent, and what nvs agent init installs"
summary: the four commands a coding agent reads the language through — `primer`, `index`, `find` and `show` — over the `Core` registry and the chapters of this reference, the `nvs check` loop that closes them, and `nvs agent init`, which writes an `AGENTS.md` stanza and what the coding agent you run needs beside it, and states no language fact
keywords: nvs agent, nvs agent primer, nvs agent index, nvs agent find, nvs agent show, nvs agent init, nvs agent hook, --agent, --all, --force, --check, --no-hooks, --json, hook, check hook, check on edit, PostToolUse, postToolUse, settings.json, .claude/settings.json, hooks.json, .cursor/hooks.json, JSON, fingerprint, coding agent, LLM, AI assistant, agent instructions, AGENTS.md, SKILL.md, .claude, Claude Code, skill, Cursor, Codex, OpenCode, GitHub Copilot, .instructions.md, adapter, pointer, stale documentation, hallucinated member, nvs check loop
---

<!-- primer -->
# nvs agent

    nvs agent index                one line per Core member, enum, exception and attribute,
                                   one per chapter of this reference and per heading in it,
                                   then one per configuration key, command, flag and error code
    nvs agent find <query>         the index lines whose symbol matches the query
    nvs agent show <symbol>        a member's card: signature, description, parameters, errors;
                                   a heading's section; a chapter's list of sections
    --json                         with index, find or show: print the answer as one JSON document

The surface a coding agent reads the language through. Every verb renders the document
`nvs meta --json` prints and the chapters this binary carries, writes nothing to disk and caches
nothing, so the binary that compiles a program is the binary that answers for it and an answer can
never describe a version that is not installed.

An index line opens with the symbol `show` resolves and continues with that member's signature:

```text
Core\IO::read(string $path): string  [fs.read]
Core\Json::decodeAs<T>(string $json, {maxDepth?: uint}): T
Core\Order  enum {Asc, Desc}
RuntimeError  exception extends Throwable
programs  chapter: Programs, files and names
programs#autoload-find-a-class-by-its-namespace  section: `autoload`: find a class by its namespace
```

The symbol is the line up to its first `(`, `<` or space, so nothing has to parse a line to get
from it back to `show` — which also accepts that leading token with a generic's `<T>` still on it.
A name in brackets after a signature is the capability the call is gated on, granted in `nvs.toml`;
a line with no bracket names a member that reaches nothing outside the program.

**A key, a command, a flag and an error code have lines too.** Each of these lines opens with the
kind of name it carries:

```text
config: [server] max_in_flight
command: nvs serve
flag: nvs serve --port
code: E0621  A `[[server.mount]]` block does not describe a valid mount.
```

The symbol is the rest of the line after `config: `, `command: ` or `flag: `, and the code alone
after `code: `. The text after a code is the first sentence of what `show` prints for it. A key is also found by its dotted name, as `server.max_in_flight`. `show` prints a
key's comment from the shipped `nvs.toml` and the key's own line, and for a command or a flag it
prints the text `--help` prints.

**A keyword is found by its heading.** The registry holds what `Core` declares, and `autoload`,
`require` and `match` are grammar, so no member is named for them. Each heading of this reference
has a line instead, whose symbol is the chapter's name, a `#`, and the heading in lowercase with
`-` between its words. `show` prints that section as the chapter has it, and `show programs` — a
chapter's name alone — prints the chapter's summary over the lines of its sections.

`find` matches that symbol rather than the whole line, case-insensitively, so a query naming a type
does not answer with every member that returns one. It is a command rather than an instruction to
grep the index, because a namespaced name loses its backslash to the shell before `grep` sees it,
and the empty result that follows is indistinguishable from a name the language does not have.
`find` prints nothing on standard output and succeeds when a query matches nothing: the index is
complete, so an empty result is the answer that no `Core` symbol, heading, key, command, flag or
code carries that word.
It is not yet the answer that the language lacks the thing — a keyword may be written under a
heading that does not name it — so standard error says what was searched and points at the chapter
map. `show` is the opposite — it was asked for one specific thing, and when it cannot resolve the
symbol it exits non-zero and prints the nearest names it does have.

**`--json` is for a tool that reads the answer.** With `--json`, `index`, `find` and `show` print
one JSON document with `schemaVersion: 1`. For `index` and `find`, `entries` is a list with one
record for each line. A record has the `symbol`, the `kind` and the `line`, and also `alias`,
`signature`, `capability`, `title` and `summary`. A key is `null` when the entry does not have that
part. The `kind` is `class`, `member`, `enum`, `exception`, `attribute`, `chapter`, `section`,
`config`, `command`, `flag` or `code`. When nothing matches, `entries` is an empty list.

For `show`, the document is the record with the parts of the card added. A member has `prose`,
`params`, `returns` and `throws`. A class or an attribute has `prose` and `members`, and an enum
has `prose` and `cases`. An exception has `parent` and `properties`, and a chapter has `sections`.
A section, a key, a command, a flag and a code have `text`. An unknown symbol gives an `error` and a
`nearest` list of records, and the command exits with a non-zero status.

So the loop is three calls and a check: `find` the name, `show` its card, write the program, then
`nvs check` it. A diagnostic points at the line the program got wrong, so the check is part of
reading the language. A call to a function that does not exist is a common case. Every callable is
a method, and the help says where the built-in methods are:

```nvs error
<?nvs

array<int> $sizes = [3, 1, 2];
echo count($sizes), "\n";
```
```output
the built-ins live under the reserved `Core` namespace
```

`nvs agent find` with the name of the task is the next call.

# nvs agent primer

    nvs agent primer

The one document to read before writing anything: the lookup protocol above, one complete worked
program with every shape in it annotated, the capability model with the smallest `nvs.toml` that
grants a file read, and a map of the chapters of this reference. It is printed on standard output,
and like every other verb here it writes nothing and caches nothing.

Each part has a heading. These are the headings, in the order the primer prints them:

```text
# Novis, for a coding agent
## nvs agent
## A complete program, annotated
## `[capabilities]`
## The chapters
```

No sentence of it is written for it. Each part is a section of a chapter of this reference, lifted
whole from the copy of that chapter this binary carries, and the counts printed beside the chapter
map are the registry's own, read at the call. So the primer describes the language this binary
compiles, and a section that stops being true stops being printed rather than becoming a lie.

# nvs agent init

    nvs agent init [--agent <name>]... [--all] [--force | --check] [--no-hooks]

Writes the files that tell a coding agent these commands exist, into the project in the working
directory. It is the only command here that writes a file. Each file points at the commands above
and says nothing else.

Two kinds of file, and both say the same thing:

- **`AGENTS.md`** is read by most agent tools. `init` adds a stanza at the end of the file, between
  two comment markers. Your own text above and below the markers stays as it is. A project with no
  `AGENTS.md` gets a new one that has only the stanza.

      <!-- nvs agent: written by `nvs agent init`, fingerprint 1a2b3c4d -->

      ## Novis

      … the four commands, and the check loop …

      <!-- /nvs agent -->

- **An adapter**, for an agent that does not always read `AGENTS.md`. An adapter is the agent's own
  header over the same text the stanza has.

`init` installs for the coding agent that runs it. Each agent sets an environment variable in the
commands it runs, and `init` reads it:

| Agent | `--agent` | Found by | Gets beside `AGENTS.md` | Check hook |
|---|---|---|---|---|
| Claude Code | `claude-code` | `CLAUDE_CODE_CHILD_SESSION` | the skill `.claude/skills/novis/SKILL.md` | in `.claude/settings.json` |
| Cursor | `cursor` | `CURSOR_AGENT` | nothing | in `.cursor/hooks.json` |
| Codex | `codex` | `CODEX_THREAD_ID` | nothing | none |
| GitHub Copilot | `copilot` | `COPILOT_AGENT`, `AI_AGENT`, `COPILOT_AGENT_SESSION_ID` or `GITHUB_COPILOT_API_TOKEN` | the instructions file `.github/instructions/novis.instructions.md` | none |
| OpenCode | `opencode` | `OPENCODE` | nothing | none |

Claude Code reads `AGENTS.md` only in a project that has no `CLAUDE.md`, so it gets a skill. Copilot
does not say which of its tools read `AGENTS.md`, so it gets its own instructions file. The other
three read `AGENTS.md`.

**The check hook.** Claude Code and Cursor can run a command after each edit. `init` adds one entry
to that agent's settings file, and the agent then runs `nvs agent hook <agent>` after it edits a
file. For a `.nvs` file the command runs `nvs check` on it and shows the agent the errors. The other
three agents have no hook that runs a plain command, so they get none. This is the entry in
`.claude/settings.json`, under `hooks.PostToolUse`:

    {"matcher": "Write|Edit|MultiEdit", "hooks": [{"type": "command", "command": "nvs agent hook claude-code"}]}

This is the entry in `.cursor/hooks.json`, under `hooks.postToolUse`. A new file also gets
`"version": 1`:

    {"command": "nvs agent hook cursor", "matcher": "Write"}

`init` never touches `.claude/settings.local.json`. It adds the entry at the end of the list and
keeps every other key and entry in the order the file had them. The file is written back with
two-space indentation. `init` finds its entry by the command alone, so you can change the matcher.
A settings file that is not a JSON object, or whose `hooks` is not an object of lists, stops `init`
with an error before it writes any file. `--force` does not change that, because the file has your
other settings.

`--no-hooks` adds no hook, and removes the entry from each file that has it, for every agent. A file
that has nothing left but the empty frame `init` started it with is deleted. With `--check`,
`--no-hooks` means that hooks are not checked.

Cursor also runs the hooks in `.claude/settings.json`. When the project has Cursor's own hook too,
`nvs agent hook claude-code` prints nothing under Cursor, so the agent sees the errors once.

`nvs agent hook <agent>` reads the agent's JSON message on standard input and takes the edited path
from it. A path that does not end in `.nvs` ends the command at once. Otherwise it checks the file as
`nvs check <file>` does, in the directory the message names, and never writes an `nvs.toml`. That
directory is the message's `cwd`, or for Cursor the first of its `workspace_roots` when there is no
`cwd`. A clean file prints nothing. For a file with errors it prints the first five errors, a line
with the number of the others, and the command that shows them all. Claude Code gets
`{"decision": "block", "reason": "<the errors>"}`: the edit stays, and the agent is told to fix the
errors. Cursor gets `{"additional_context": "<the errors>"}`. Warnings are not shown. The command
always exits with status 0, so a hook never stops the agent.

The first line `init` prints names the agent it chose, and the variable it found. When you run
`init` yourself in a terminal, no agent is found, and only the stanza is written. `--agent <name>`
installs for that agent instead, and you can give it more than once. `--all` installs for every
agent in the table.

A file that an earlier `init` wrote is kept up to date, whichever agent runs `init` now. So every
person on a team gets the same files. An earlier `init` also wrote `.cursor/rules/novis.mdc` for
Cursor. Cursor reads `AGENTS.md`, so `init` now deletes that file and prints `removed <path>`. If
somebody changed the file, `init` leaves it and prints a note.

**No file states a language fact.** It has no signature, no type and no error message. A copy of a
fact gets old when the language changes, and the agent that reads it cannot tell. So the files
only say which commands to run, and the installed `nvs` gives the answers.

**Run it again after you update `nvs`.** Each file has a fingerprint in its marker. The fingerprint
is computed from the text `init` wrote, so `init` can see whether somebody changed the file:

- A file that is missing is written, and `init` prints `wrote <path>`. A hook that is missing is
  added, and `init` prints `added hook to <path>`. With `--no-hooks` it prints
  `removed hook from <path>`.
- A file that nobody changed, but that an older `nvs` wrote, is replaced with the current text.
  `init` prints `updated <path>`.
- A file that somebody changed is not touched. `init` prints an error that names the file, exits
  with a non-zero status, and writes no file at all. `--force` replaces the changed file.
- When there is nothing to write, `init` prints `up to date`.

Line endings do not count, so a checkout with CRLF line endings has the same fingerprint.
`nvs agent primer` also prints one line on standard error when `init` would change a file: one
that an older `nvs` wrote, or one that the agent reading the primer needs and the project lacks.
A missing hook does not count. So a project that you set up with `--no-hooks` gets no note, and
the next `init` without `--no-hooks` adds the hook.

`--check` writes nothing. It prints `missing`, `outdated`, `edited` or `retired` and the path for
each file that is not current, and `missing hook` and the path for a hook the agent lacks. Then it
exits with a non-zero status. When every file is current it
prints `up to date` and succeeds. You can run it in CI, where no agent is found, so it checks the
stanza and the files that are already there.

These are three runs in a terminal, in a project that has no `AGENTS.md`. The second run writes
nothing. The third run has `--all`, so it installs for every agent:

```text
$ nvs agent init
agent: none detected; pass --agent <name> to install an agent's own files
wrote AGENTS.md
$ nvs agent init
agent: none detected; pass --agent <name> to install an agent's own files
up to date
$ nvs agent init --all
agent: every one (--all)
wrote .claude/skills/novis/SKILL.md
wrote .github/instructions/novis.instructions.md
added hook to .claude/settings.json
added hook to .cursor/hooks.json
```

# A worked session

An agent that holds nothing but this binary, asked to print how long a name is. It reads the primer
once, then looks for the member by a name it knows from another language:

```text
$ nvs agent find strlen
$
```

Nothing on standard output, and the exit status is `0`: the index is complete, so no `Core` symbol
and no heading has that name. It searches for the operation instead of the spelling:

```text
$ nvs agent find length
Core\Str::length(string $s): uint
Core\Bytes::length(bytes $b): uint
```

Two, and the card says which one is meant:

```text
$ nvs agent show 'Core\Str::length'
Core\Str::length(string $s): uint

Counts the graphemes in `$s` — user-perceived characters, the unit every `Core\Str` member counts in — so a combining sequence counts once and this is never a byte count.

params:
  $s  The string to measure.

returns:
  The grapheme count; `0` for the empty string.
```

Had it written the program first, `nvs check` would have stopped at the call. The free function
does not exist, and the help says the built-in methods are on `Core` classes:

```nvs error
<?nvs

string $name = "Zoë";
echo strlen($name), "\n";
```
```output
the built-ins live under the reserved `Core` namespace
```

Either way it arrives at one program, and `nvs check` accepts it:

```nvs
<?nvs

string $name = "Zoë";
echo Core\Str::length($name), "\n";
```
```output
3
```

Asked next whether classes load themselves, it looks for the word it already knows, which is a keyword here and
no member's name. Two lines come back: a heading, and a flag of `nvs check` that has the word in
its name. The heading is the one it wants, and the same `show` prints the section:

```text
$ nvs agent find autoload
programs#autoload-find-a-class-by-its-namespace  section: `autoload`: find a class by its namespace
flag: nvs check --autoload-map
$ nvs agent show 'programs#autoload-find-a-class-by-its-namespace'
programs#autoload-find-a-class-by-its-namespace  section: `autoload`: find a class by its namespace

# `autoload`: find a class by its namespace

`autoload` declares a rule that maps a namespace prefix to a directory, so ordinary code names a
class and never a file. …
```
