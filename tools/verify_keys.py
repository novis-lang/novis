#!/usr/bin/env python3
"""What each `tools/verify.py` step reads, and one key per step over exactly that.

`verify.py` answers a step from its green cache when the step's key is the one it was green
under. This file is the key: which files a step reads, and -- for Rust -- how much of each file
it reads. *Why a step whose inputs did not change is not run* in `verify.py`'s module doc is the
reasoning; this is the table and the scanner behind it.

## A step reads a partition, not the tree

`STEP_READS` names each step's partition. The part most steps share is `binary`: everything that
decides how `target/debug/nvs` behaves -- the toolchain, the manifests and locks, every `.rs` file
at its *code* tier, every other file under `crates/`, the reference chapters `nvs-cli` embeds, and
the files a `build.rs` reads. A case tree's key is `binary` plus its own directory, so a file under
`tests/hostile/` is not an input of `conformance`, and a chapter under `docs/reference/` is.

## A `.rs` file has three readers

- **raw** -- the bytes. `fmt` reads these, and so do the script steps that grep doc comments, and
  so does every test binary: a policy test here reads source as text (`include_str!("cache.rs")`,
  a walk over `crates/*/src`), and for such a test a comment is an input like any other.
- **full** -- the code and the text of every comment, with the layout between tokens removed.
  `clippy` lints doc comments and `// SAFETY:` comments, and a doc-test is a doc comment.
- **code** -- the tokens alone. `rustc` reads nothing else, so this is what `build` and the
  behaviour of the binary hang on. A doc comment leaves one placeholder per run of them: whether
  an item has one can decide a build, what it says cannot.

Layout is removed conservatively: a run of whitespace is kept, as one space, between two words
and between two operator characters, so `& &x` and `&&x` never share a key. It is dropped
everywhere else -- where a word meets punctuation, and beside a bracket, a comma or a semicolon,
none of which can join a neighbour into another token. String, byte-string, raw-string and character literals are lifted out
before any of that and hashed verbatim. A `.rs` file that some other file embeds with
`include_str!` or `include_bytes!` is data, and is raw at every tier.

What the code tier does not see is a line number. A formatting-only edit moves `line!()` and the
location in a panic message, and a step answered from the cache was proved against the old ones.
No test in this tree pins a Rust line number in its expectation; one that starts to belongs to a
test binary, and those read raw.

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
SCANNER = "1"

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


def scan(text):
    """`(code, literals, comments)`: the tokens with their layout removed, the literals lifted out
    of them in order, and every comment's text in order. The module doc's *A `.rs` file has three
    readers* is what each is for."""
    code, literals, comments = [], [], []
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
            comments.append(body)
            code.append(" \x01 " if is_doc else " ")
        elif kind == "doc":
            comments.append(m.group().rstrip())
            code.append(" \x01 ")
        elif kind == "line":
            comments.append(m.group().rstrip())
            code.append(" ")
        else:
            literals.append(m.group())
            code.append("\x02")
        pos = end
    flat = _GLUE.sub("", _WS.sub(" ", "".join(code)).strip())
    return _DOC_RUN.sub("\x01", flat), literals, comments


def tiers(text):
    """`(code, full)` digests of one `.rs` file's text."""
    code, literals, comments = scan(text)
    h = hashlib.blake2b(digest_size=16)
    h.update(SCANNER.encode())
    h.update(code.encode("utf-8", "replace"))
    for lit in literals:
        h.update(b"\0")
        h.update(lit.encode("utf-8", "replace"))
    code_digest = h.hexdigest()
    h.update(b"\1")
    for comment in comments:
        h.update(b"\0")
        h.update(comment.encode("utf-8", "replace"))
    return code_digest, h.hexdigest()


# ------------------------------------------------------------------ the tree


def digest_of(path):
    """One file's raw digest."""
    return hashlib.blake2b(Path(path).read_bytes(), digest_size=16).hexdigest()


def rustc_version():
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")


def input_paths():
    """Every file a step reads, sorted, as ROOT-relative posix strings."""
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
            dirnames[:] = [d for d in dirnames if d not in NOT_INPUTS]
            rel = Path(dirpath).relative_to(ROOT)
            seen.update((rel / f).as_posix() for f in filenames)
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
    """One reading of every input: each file's raw digest, each `.rs` file's two tiers, and the
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
            if not (isinstance(got, list) and len(got) == 2):
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
            return self.tier[rel][0 if tier == "code" else 1]
        return self.raw[rel]

    def part(self, pred, tier="raw"):
        return [(rel, self.digest(rel, tier)) for rel in sorted(self.raw) if pred(rel)]

    def binary(self):
        """What decides how `target/debug/nvs` is built and behaves."""
        return ([("rustc", self.toolchain)]
                + self.part(is_manifest)
                + self.part(is_rust, "code")
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
    "fuzz-lock": lambda t: [("rustc", t.toolchain)] + t.part(is_manifest),
    "build": lambda t: t.binary(),
    "test": _everything,
    # The one test job that reads no file as text: a doc-test is a doc comment, compiled.
    "test:doc": lambda t: t.binary() + t.part(is_rust, "full"),
    "conformance": lambda t: t.binary() + t.part(under("tests/conformance")),
    "differential": lambda t: t.binary() + t.part(under("tests/differential")),
    "reference": lambda t: t.binary() + t.part(_only("tools/reference.py")),
    "clippy": lambda t: t.binary() + t.part(is_rust, "full"),
    "extension": lambda t: t.binary() + t.part(under("editors")),
    "doc": lambda t: t.binary() + t.part(is_rust, "full"),
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
