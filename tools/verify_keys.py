#!/usr/bin/env python3
"""What each `tools/verify.py` step reads, and one key per step over exactly that.

`verify.py` answers a step from its green cache when the step's key is the one it was green
under. This file is the key: which files a step reads, and -- for Rust -- how much of each file
it reads. *Why a step whose inputs did not change is not run* in `verify.py`'s module doc is the
reasoning; this is the table and the scanner behind it.

## A step reads a partition, not the tree

`STEP_READS` names each step's partition. The part most steps share is `binary`: everything that
decides how `target/debug/nvs` behaves -- the toolchain, the manifests and locks, every `.rs` file
at its *code* or its *shipped* tier, every other file under `crates/`, the reference chapters
`nvs-cli` embeds, and the files a `build.rs` reads. A case tree's key is `binary` plus its own
directory, so a file under `tests/hostile/` is not an input of `conformance`, and a chapter under
`docs/reference/` is.

## A `.rs` file has four readers

- **raw** -- the bytes. `fmt` reads these, and so do the script steps that grep doc comments, and
  so does every test binary: a policy test here reads source as text (`include_str!("cache.rs")`,
  a walk over `crates/*/src`), and for such a test a comment is an input like any other.
- **docs** -- the code and the text of every doc comment, with the layout between tokens removed.
  `clippy` and rustdoc lint doc comments, and a doc-test is one. A plain comment is not in this
  tier because nothing here reads one: no crate enables a lint that does (`grep -rn
  'undocumented_unsafe_blocks\|safety_comment\|clippy::restriction' Cargo.toml crates benches` is
  the evidence, and the grep to re-run before enabling one).
- **code** -- the tokens alone. `rustc` reads nothing else, so this is what `build` and the
  behaviour of the binary hang on. A doc comment leaves one placeholder per run of them: whether
  an item has one can decide a build, what it says cannot. The one thing `rustc` does read in a
  comment is a bidirectional-text character, which it denies, so a file with one in any comment
  has every comment folded into this tier. `clippy --all-targets` and the test binaries compile
  with `cfg(test)`, so every token is theirs.
- **shipped** -- the code tier without the body of any inline `#[cfg(test)] mod name { ... }`.
  `target/debug/nvs` is built without `cfg(test)`, and `rustc` removes such a module before it
  resolves a name, so no token inside one can reach the binary. This is what a step that only
  RUNS the binary hangs on -- the `.nvst` trees, `reference`, `extension` -- and a `#[test]`
  added to a source file's test module reaches none of them. The header stays in the tier, so
  whether a file has the module is still in the key. `build` stays on the code tier: a removed
  module must still parse, and `build`'s key is also what says the test binaries on disk are the
  right ones. Only that one form is removed. A `#[cfg(test)]` on a function, an `impl`, a `use`
  or an out-of-line `mod name;`, and a `cfg(any(test, ...))` or a `cfg_attr`, are hashed as the
  code they are, which is the wide direction; `_TEST_MOD` is the form, and a module whose braces
  do not close is left whole.

Layout is removed conservatively: a run of whitespace is kept, as one space, between two words
and between two operator characters, so `& &x` and `&&x` never share a key. It is dropped
everywhere else -- where a word meets punctuation, and beside a bracket, a comma or a semicolon,
none of which can join a neighbour into another token. String, byte-string, raw-string and character literals are lifted out
before any of that and hashed verbatim. A `.rs` file that some other file embeds with
`include_str!` or `include_bytes!` is data, and is raw at every tier.

What the code and shipped tiers do not see is a line number. A formatting-only edit, or a test
added to a module with code below it, moves `line!()` and the location in a panic message, and a
step answered from the cache was proved against the old ones.
The steps that can be answered that way run the binary over `tests/conformance`,
`tests/differential` and the reference chapters, and no expectation in those names a Rust line --
`\.rs:\d+` matches there only in a case's prose comment. A Rust test that pins one is in a test
binary, and those read raw.

The tiers are memoized against a file's raw digest in `.agent-tmp/verify-norms.json`, so a run
re-scans only the files that changed. `SCANNER` is folded into every tiered digest: a change to the
scanner is a change to every key it produced.
"""
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"
NORMS = TMP / "verify-norms.json"
SCANNER = "3"
TIERS = ("shipped", "code", "docs")

# Everything cargo reads, relative to ROOT. Directories are walked in full -- a `.nvst`
# fixture, an insta `.snap` and a `Cargo.toml` all change what the steps will answer.
INPUT_DIRS = ("crates", "benches", "tests", "examples", "editors", "docs/reference")
INPUT_FILES = ("Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml",
               "clippy.toml", ".clippy.toml", ".cargo/config.toml",
               # The `fuzz-lock` step's own manifest. Everything else that step resolves against
               # is a manifest under `crates/` or the root lock, both hashed already;
               # `fuzz/Cargo.lock` is what it writes, so it is derived and stays out.
               "fuzz/Cargo.toml",
               # The steps that are a script rather than `cargo`. Their verdict changes when
               # the script does -- a crate added to `lints.py`'s roster, a reader counted a
               # third way in `directives.py`, a chapter rule changed in `reference.py`. The rest
               # of `tools/` is deliberately not an input: `loop.py` and friends change most
               # sessions and change nothing these steps would say.
               "tools/lints.py", "tools/reference.py", "tools/directives.py",
               # What decides which test binaries the `test` step runs, and which of them may
               # be wide: an edit to either is a reason to ask the step again.
               "tools/impact.py", "tools/data/impact-wide.txt",
               # The third file `reference.py` reads, and the only one outside `docs/reference/`:
               # every row of the migration table is rendered into `docs/novis.md`. It is also
               # one of the two files `crates/nvs-stdlib/build.rs` reads; the inventory beside it
               # is the other, and the two after that are `crates/nvs-cli/build.rs`'s.
               "docs/spec/02-php-migration.md", "tools/data/php-builtins.txt",
               "LICENSE", "THIRD-PARTY-LICENSES.txt")
# Directories under an INPUT_DIR that are output or a package cache, never an input. `target` is
# cargo's; the other three belong to `editors/vscode` and between them hold tens of thousands of
# files, which would make the green cache's own hash the slowest thing in `verify.py`.
NOT_INPUTS = {"target", "node_modules", "out", ".vscode-test"}
# What `tools/owners.py` reads beside the doc comments under `crates/`: the plan that says which
# milestones are still ahead, the chain that says which goals exist, and the two scripts that read
# them. No other step reads these, so they are in no other step's key.
OWNERS_READS = ("docs/implementation-plan.md", "docs/plan", "docs/agent/goals",
                "tools/owners.py", "tools/goals.py")
# What a `build.rs` reads from outside `crates/`: `crates/nvs-stdlib/build.rs` the first two,
# `crates/nvs-cli/build.rs` the other two. An embedded file is found by the scan instead.
BUILD_READS = ("docs/spec/02-php-migration.md", "tools/data/php-builtins.txt",
               "LICENSE", "THIRD-PARTY-LICENSES.txt")

MANIFEST_NAMES = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "config.toml"}


# ------------------------------------------------------------------ the Rust scanner

_TOKEN = re.compile(r"""
    (?P<doc>//(?:/(?!/)|!)[^\n]*)
  | (?P<line>//[^\n]*)
  | (?P<block>/\*)
  | (?P<raw>(?<![A-Za-z0-9_])(?:b|c)?r(?P<hashes>\#*)"[\s\S]*?"(?P=hashes))
  | (?P<str>(?:(?<![A-Za-z0-9_])(?:b|c))?"(?:\\[\s\S]|[^"\\])*")
  | (?P<chr>(?:(?<![A-Za-z0-9_])b)?'(?:\\(?:u\{[^}\n]*\}|x[0-9a-fA-F]{2}|[^\n])|[^\\'\n])')
""", re.X)
_NEST = re.compile(r"/\*|\*/")
_WS = re.compile(r"\s+")
_GLUE = re.compile(r"(?<=\w) (?=\W)|(?<=\W) (?=\w)"
                   r"|(?<=[()\[\]{},;\x01\x02]) | (?=[()\[\]{},;\x01\x02])")
_DOC_RUN = re.compile("\x01(?: ?\x01)+")
_INCLUDE = re.compile(r'include_(?:str|bytes)!\(\s*"([^"\n]+)"')
# An inline test module's header as `scan` leaves it: the attribute, any doc comment or bracket-free
# attribute after it, an optional visibility, and the opening brace. Nothing looser is matched.
_TEST_MOD = re.compile(r"#\[cfg\(test\)\](?:\x01|#\[[^\[\]]*\])*(?:pub(?:\([^()]*\))? ?)?mod \w+\{")
_BRACE = re.compile(r"[{}]")
# What `text_direction_codepoint_in_comment` denies: the embeddings, overrides and isolates.
_BIDI = re.compile("[‪-‮⁦-⁩]")


def scan(text):
    """`(code, literals, docs, plain)`: the tokens with their layout removed, the literals lifted
    out of them in order, and the text of every doc comment and of every other comment, in order.
    The module doc's *A `.rs` file has three readers* is what each is for."""
    code, literals, docs, plain = [], [], [], []
    pos = 0
    while True:
        m = _TOKEN.search(text, pos)
        if m is None:
            code.append(text[pos:])
            break
        code.append(text[pos:m.start()])
        end = m.end()
        kind = m.lastgroup
        if kind == "hashes":
            kind = "raw"
        if kind == "block":
            # Block comments nest, which no regular expression follows. An unclosed one runs to
            # the end of the file, as it does for rustc.
            depth, at = 1, end
            while depth:
                n = _NEST.search(text, at)
                if n is None:
                    at = len(text)
                    break
                depth += 1 if n.group() == "/*" else -1
                at = n.end()
            end = at
            body = text[m.start():end]
            is_doc = (body.startswith("/*!")
                      or (body.startswith("/**") and not body.startswith(("/***", "/**/"))))
            (docs if is_doc else plain).append(body)
            code.append(" \x01 " if is_doc else " ")
        elif kind == "doc":
            docs.append(m.group().rstrip())
            code.append(" \x01 ")
        elif kind == "line":
            plain.append(m.group().rstrip())
            code.append(" ")
        else:
            literals.append(m.group())
            code.append("\x02")
        pos = end
    flat = _GLUE.sub("", _WS.sub(" ", "".join(code)).strip())
    return _DOC_RUN.sub("\x01", flat), literals, docs, plain


def without_test_modules(code, literals):
    """`scan`'s `code` and `literals` with the body of every inline `#[cfg(test)] mod` removed,
    and the literals that were inside one with it. The header and both braces stay. Comments and
    literals are already out of `code`, and a token tree's braces balance even inside a macro, so
    counting them finds the module's end; one that never closes is left as it is."""
    out, kept, pos, lit = [], [], 0, 0
    while True:
        m = _TEST_MOD.search(code, pos)
        if m is None:
            break
        depth, close = 1, None
        for b in _BRACE.finditer(code, m.end()):
            depth += 1 if b.group() == "{" else -1
            if depth == 0:
                close = b.start()
                break
        if close is None:
            break
        out.append(code[pos:m.end()])
        before = code.count("\x02", pos, m.end())
        kept.extend(literals[lit:lit + before])
        lit += before + code.count("\x02", m.end(), close)
        pos = close
    out.append(code[pos:])
    kept.extend(literals[lit:])
    return "".join(out), kept


def tiers(text):
    """`(shipped, code, docs)` digests of one `.rs` file's text."""
    code, literals, docs, plain = scan(text)
    comments = docs + plain if any(_BIDI.search(c) for c in docs + plain) else []

    def tokens(code, literals):
        h = hashlib.blake2b(digest_size=16)
        h.update(SCANNER.encode())
        h.update(code.encode("utf-8", "replace"))
        for lit in literals:
            h.update(b"\0")
            h.update(lit.encode("utf-8", "replace"))
        for comment in comments:
            h.update(b"\2")
            h.update(comment.encode("utf-8", "replace"))
        return h

    shipped_digest = tokens(*without_test_modules(code, literals)).hexdigest()
    h = tokens(code, literals)
    code_digest = h.hexdigest()
    h.update(b"\1")
    for comment in docs:
        h.update(b"\0")
        h.update(comment.encode("utf-8", "replace"))
    return shipped_digest, code_digest, h.hexdigest()


# ------------------------------------------------------------------ the tree


def digest_of(path):
    """One file's raw digest."""
    return hashlib.blake2b(Path(path).read_bytes(), digest_size=16).hexdigest()


def rustc_version():
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")


def git_ignored():
    """What git ignores: a file path, or a wholly ignored directory with a trailing `/`. Empty
    when git cannot answer, and then everything is hashed, which is the wide direction."""
    try:
        p = subprocess.run(["git", "ls-files", "--others", "--ignored", "--exclude-standard",
                            "--directory"], cwd=ROOT, capture_output=True, encoding="utf-8",
                           errors="replace")
    except OSError:
        return set()
    return {line for line in p.stdout.split("\n") if line} if p.returncode == 0 else set()


def input_paths():
    """Every file a step reads, sorted, as ROOT-relative posix strings.

    What git ignores is not one of them, which is `tools/loop.py`'s rule for its own memo and for
    the same reason: every rule in `.gitignore` is build output, a cache or machine-local state,
    and two of those files are written by the checks themselves -- `editors/vscode/nvs.vsix` by
    the driver's sweep and `tests/db/queue-sqlite.db` by a test -- so hashing them made a step's
    own run the edit that staled it."""
    ignored = git_ignored()
    seen = set()
    for name in INPUT_FILES:
        if (ROOT / name).is_file():
            seen.add(name)
    for top in INPUT_DIRS + OWNERS_READS:
        base = ROOT / top
        if base.is_file():
            seen.add(top)
        if not base.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            rel = Path(dirpath).relative_to(ROOT).as_posix()
            dirnames[:] = [d for d in dirnames
                           if d not in NOT_INPUTS and f"{rel}/{d}/" not in ignored]
            seen.update(f"{rel}/{f}" for f in filenames if f"{rel}/{f}" not in ignored)
    return sorted(seen)


def under(*tops):
    return lambda rel: any(rel == top or rel.startswith(top + "/") for top in tops)


def is_manifest(rel):
    name = rel.rsplit("/", 1)[-1]
    if name == "config.toml":
        return rel.endswith(".cargo/config.toml")
    return name in MANIFEST_NAMES


def is_rust(rel):
    return rel.endswith(".rs")


class Tree:
    """One reading of every input: each file's raw digest, each `.rs` file's `TIERS`, and the
    set of files some `.rs` file embeds. Raises `OSError` if a file cannot be read; `verify.py`
    then runs every step for real."""

    def __init__(self):
        self.toolchain = rustc_version()
        self.raw, self.tier, self.embedded, self._keys = {}, {}, set(), {}
        try:
            memo = json.loads(NORMS.read_text(encoding="utf-8"))
            memo = memo.get(SCANNER, {}) if isinstance(memo, dict) else {}
        except (OSError, ValueError):
            memo = {}
        kept = {}
        for rel in input_paths():
            data = (ROOT / rel).read_bytes()
            digest = hashlib.blake2b(data, digest_size=16).hexdigest()
            self.raw[rel] = digest
            if not is_rust(rel):
                continue
            text = data.decode("utf-8", "replace")
            for target in _INCLUDE.findall(text):
                joined = os.path.normpath(os.path.join(os.path.dirname(rel), target))
                self.embedded.add(joined.replace("\\", "/"))
            got = memo.get(digest)
            if not (isinstance(got, list) and len(got) == len(TIERS)):
                got = list(tiers(text))
            kept[digest] = got
            self.tier[rel] = got
        # A file embedded from outside the walked directories is still an input of the binary.
        for rel in sorted(self.embedded - set(self.raw)):
            path = ROOT / rel
            if not rel.startswith("..") and path.is_file():
                self.raw[rel] = hashlib.blake2b(path.read_bytes(), digest_size=16).hexdigest()
        if kept != memo:
            try:
                TMP.mkdir(exist_ok=True)
                NORMS.write_text(json.dumps({SCANNER: kept}), encoding="utf-8", newline="\n")
            except OSError:
                pass

    def digest(self, rel, tier):
        if tier != "raw" and rel in self.tier and rel not in self.embedded:
            return self.tier[rel][TIERS.index(tier)]
        return self.raw[rel]

    def part(self, pred, tier="raw"):
        return [(rel, self.digest(rel, tier)) for rel in sorted(self.raw) if pred(rel)]

    def binary(self, tier="code"):
        """What decides how `target/debug/nvs` is built and behaves. `code` is every token the
        compiler is handed; `shipped` is for a step that only runs the binary it produced."""
        return ([("rustc", self.toolchain)]
                + self.part(is_manifest)
                + self.part(is_rust, tier)
                + self.part(lambda r: under("crates", "docs/reference")(r)
                            and not is_rust(r) and not is_manifest(r))
                + self.part(lambda r: r in BUILD_READS or r in self.embedded))

    def key(self, step, scope=None):
        """The key `step` is green under, or None for a step this file does not describe -- which
        `verify.py` takes as "always run"."""
        reads = STEP_READS.get(step)
        if reads is None:
            return None
        if (step, scope) not in self._keys:
            self._keys[(step, scope)] = self._key(step, scope, reads)
        return self._keys[(step, scope)]

    def _key(self, step, scope, reads):
        h = hashlib.blake2b(digest_size=16)
        h.update(f"{step}\0{scope or ''}\0".encode())
        for label, digest in reads(self):
            h.update(label.encode("utf-8"))
            h.update(b"\0")
            h.update(digest.encode("utf-8", "replace"))
            h.update(b"\0")
        return h.hexdigest()


def _only(*names):
    return lambda rel: rel in names


def _everything(t):
    # The test binaries: a policy test may read any file the old whole-tree key covered, as text.
    return [("rustc", t.toolchain)] + t.part(lambda r: not under(*OWNERS_READS)(r))


#: Each step's partition. A step that is absent is never answered from the cache.
STEP_READS = {
    "fmt": lambda t: t.part(is_rust) + t.part(is_manifest) + t.part(_only("rustfmt.toml")),
    "lints": lambda t: t.part(is_manifest) + t.part(_only("tools/lints.py")),
    "directives": lambda t: t.part(under("crates", "benches")) + t.part(_only("tools/directives.py")),
    "template": lambda t: t.part(under("crates", "benches")) + t.part(_only("tools/directives.py")),
    "owners": lambda t: t.part(under("crates")) + t.part(under(*OWNERS_READS)),
    "nv": lambda t: t.part(under("tools/nv"))
    + t.part(_only("package.json", "bun.lock", "tsconfig.json")),
    "fuzz-lock": lambda t: [("rustc", t.toolchain)] + t.part(is_manifest),
    "build": lambda t: t.binary(),
    "test": _everything,
    # The one test job that reads no file as text: a doc-test is a doc comment, compiled.
    "test:doc": lambda t: t.binary() + t.part(is_rust, "docs"),
    "conformance": lambda t: t.binary("shipped") + t.part(under("tests/conformance")),
    "differential": lambda t: t.binary("shipped") + t.part(under("tests/differential")),
    "reference": lambda t: t.binary("shipped") + t.part(_only("tools/reference.py")),
    "clippy": lambda t: t.binary() + t.part(is_rust, "docs"),
    "extension": lambda t: t.binary("shipped") + t.part(under("editors")),
    "doc": lambda t: t.binary() + t.part(is_rust, "docs"),
}


def main():
    """`python tools/verify_keys.py`: every step's key for the tree as it stands. Two runs either
    side of an edit say which steps that edit reaches."""
    tree = Tree()
    for step in STEP_READS:
        print(f"{step:<13} {tree.key(step)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
