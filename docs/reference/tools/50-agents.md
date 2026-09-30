---
id: agents
title: "Coding agents: nvs agent, and what nvs agent init installs"
summary: the four commands a coding agent reads the language through — `primer`, `index`, `find` and `show` — over the `Core` registry and the chapters of this reference, the `nvs check` loop that closes them, and `nvs agent init`, which writes an `AGENTS.md` stanza and one adapter per harness and states no language fact
keywords: nvs agent, nvs agent primer, nvs agent index, nvs agent find, nvs agent show, nvs agent init, --all, coding agent, LLM, AI assistant, agent instructions, AGENTS.md, SKILL.md, .claude, Claude Code, skill, adapter, pointer, stale documentation, hallucinated member, nvs check loop
---

<!-- primer -->
# nvs agent

    nvs agent index                one line per Core member, enum, exception and attribute,
                                   one per chapter of this reference and per heading in it,
                                   then one per configuration key, command, flag and error code
    nvs agent find <query>         the index lines whose symbol matches the query
    nvs agent show <symbol>        a member's card: signature, description, parameters, errors;
                                   a heading's section; a chapter's list of sections

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
code: E0621
```

The symbol is the rest of the line after `config: `, `command: ` or `flag: `, and the code alone
after `code: `. A key is also found by its dotted name, as `server.max_in_flight`. `show` prints a
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

So the loop is three calls and a check: `find` the name, `show` its card, write the program, then
`nvs check` it. A diagnostic names the spelling this language wants at the place the program got it
wrong, which makes the check part of reading the language rather than an alternative to it. A call
to a PHP function is the common case. The help names what replaces that function: a `Core` member,
an operator, or nothing.

```nvs error
<?nvs

array<int> $sizes = [3, 1, 2];
echo count($sizes), "\n";
```
```output
PHP's `count` is `Core\Arr::count` here
```

A name the help has no single answer for gets the general sentence, and `find` is the next call.

# nvs agent primer

    nvs agent primer

The one document to read before writing anything: the lookup protocol above, one complete worked
program with every shape in it annotated, the capability model with the smallest `nvs.toml` that
grants a file read, the refusal tables — the PHP spellings this language does not have, each with
the code it is refused under — and a map of the chapters of this reference. It is printed on
standard output, and like every other verb here it writes nothing and caches nothing.

Each part has a heading. These are the headings, in the order the primer prints them. The `…`
replaces the other tables of PHP syntax, which have one heading each:

```text
# Novis, for a coding agent
## nvs agent
## A complete program, annotated
## `[capabilities]`
## Files, tags and names
…
## The chapters
```

No sentence of it is written for it. Each part is a section of a chapter of this reference, lifted
whole from the copy of that chapter this binary carries, and the counts printed beside the chapter
map are the registry's own, read at the call. So the primer describes the language this binary
compiles, and a section that stops being true stops being printed rather than becoming a lie.

# nvs agent init

    nvs agent init [--all]

Installs the surface into the project in the working directory, by writing the pointers that tell a
coding agent these commands exist. It is the only verb here that writes a file, and what it writes
is a pointer at the four above rather than anything one of them would answer.

Two kinds of file, and both say the same thing:

- **`AGENTS.md`** — harness-neutral, and read by most agent harnesses. It gains a stanza delimited
  by the two comment markers `init` owns, at the foot of whatever the file already said, so a
  project's own instructions keep the opening of their own document. A project with no `AGENTS.md`
  gets one holding the stanza alone.

      <!-- nvs agent: written by `nvs agent init` -->

      ## Novis

      … the four commands, and the check loop …

      <!-- /nvs agent -->

- **One adapter per harness**, in the place that harness looks: a Claude Code skill at
  `.claude/skills/novis/SKILL.md`, written when the tree has a `.claude/` directory. An adapter is
  that harness's own header over the same protocol the stanza carries, so what it says is fixed and
  only where it goes is the harness's. `--all` writes every adapter whether or not the tree shows
  the harness, which is what a project sets up for contributors on other tools.

**No adapter states a language fact.** Not a signature, not a type, not a refusal. A language fact
written into a pointer is a copy of an answer, and a copy is read by an agent that has no way to
know it is old — which is the failure the commands above exist to remove. A pointer says where to
ask; the binary answers.

**Re-running rewrites nothing.** A stanza or an adapter that still reads as this binary writes it is
left alone, and line endings do not count: a checkout with CRLF line endings is the same text.
`init` prints `wrote <path>` for each file it created, and `up to date` when it created none. A
stanza or adapter holding anything else is refused — the message names the file and says to
delete the block and run the command again, the exit status is non-zero, and nothing on disk
changes. An upgrade and an edit somebody made on purpose look identical from the file, so both take
the same answer, which is the one that cannot destroy the reader's own sentence.

These are three runs in a project that has no `AGENTS.md` and no `.claude/` directory. The second
run creates nothing. The third run has `--all`, so it creates the Claude Code skill:

```text
$ nvs agent init
wrote AGENTS.md
$ nvs agent init
up to date
$ nvs agent init --all
wrote .claude/skills/novis/SKILL.md
```

# A worked session

An agent that holds nothing but this binary, asked to print how long a name is. It reads the primer
once, then looks for the member by the PHP name it already knows:

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

Had it written the program first, `nvs check` would have said the same thing at the place it got it
wrong — the free function does not exist, and the help names the member that replaces it:

```nvs error
<?nvs

string $name = "Zoë";
echo strlen($name), "\n";
```
```output
PHP's `strlen` is `Core\Str::length` here
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

Asked next whether classes load themselves, it looks for the PHP word, which is a keyword here and
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
