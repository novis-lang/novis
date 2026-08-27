#!/usr/bin/env python3
"""Rename the project: MWL -> Novis, `mwl` -> `nvs`, in one call.

    python tools/rename-project.py              # dry run: the full report, changes nothing
    python tools/rename-project.py --apply      # do it, then `cargo fmt --all`, then audit
    python tools/rename-project.py --audit      # audit only: is anything still named `mwl`?

    python tools/rename-project.py --apply --tracked-only   # skip examples/ and .agent-tmp/
    python tools/rename-project.py --apply --no-fmt         # skip the trailing `cargo fmt`

This is a one-shot tool. It is committed because the *decisions* below are the deliverable --
re-deriving them from a fresh `grep` is what would get one of them wrong -- and because it is
idempotent, so it can be re-run over a branch that was written before the rename landed.

Why a script and not `sed`. `mwl` is three letters that appear in four capitalisations and in
two grammatical roles, and the roles disagree about what the replacement is:

  * As a *token* -- `mwl-ir`, `mwl_runtime`, `.mwlt`, `MWL_JOBS`, `MwlStr`, `<?mwl`, `mwl.toml` --
    it becomes `nvs`, letter for letter, carrying case per character (m->n, w->v, l->s). This is
    length-preserving, which is why every diagnostic's caret alignment survives it untouched.
  * As a *word* in prose -- "MWL is a JIT-compiled language", "an MWL-native format" -- it becomes
    "Novis", because that is the project's name. Only the all-caps spelling is prose: nobody
    writes the name lowercase, so a standalone `mwl` is the CLI binary and stays a token.

A `sed` doing one of those breaks the other, and a `sed` doing both in the wrong order
double-applies. Worse, four fixtures feed the project name *into* a case-mapping function and
assert the result -- `Core\\Str::lowerFirst("MWL")` expects `mWL` -- so in those files the prose
rule would rewrite the input and leave the expectation unreachable. They are listed in
DATA_FILES and take the token rule alone.

The survey behind those choices, run over the pre-rename tree:

  * 16,018 occurrences in content, 1,426 in paths, and **not one false positive**. No English
    word contains `mwl`; the only letter-adjacent forms were `TagMwl` (the `OpenTagMwl` token)
    and `libmwl_stdlib` in a comment. So the token rule can be unconditional.
  * Exactly four capitalisations: `mwl` 13,326, `MWL` 1,644, `Mwl` 1,046, `mWL` 2. The last is
    not a typo -- it is the expected output of `lowerFirst("MWL")`, and per-character case
    transfer maps it to `nVS`, which is what `lowerFirst("NVS")` returns. That is the whole
    reason case is carried per character rather than by a three-way if.
  * `MWL` is never a code identifier -- every standalone occurrence is a comment, a doc comment,
    a Cargo `description`, a diagnostic's help text, or Markdown. Grepped for `const/static/fn/
    struct/enum/let/mod/use/type MWL`, `MWL::` and `MWL =`: no hits.
  * The 68 standalone `MWL`s sitting next to a `|` are all Markdown table cells, not the
    column-aligned caret art of a rendered diagnostic. Nothing in the tree depends on the word
    being three characters wide, which is what makes the 3->5 prose rule safe.
  * Diagnostic help strings ("MWL has no references: ADR 0031 ...") live in Rust *and* are
    asserted byte-for-byte in `.mwlt` expectation blocks. Both sides take the prose rule, so
    they stay in sync -- which is why `.mwlt` files are **not** excluded wholesale.

Out of scope on purpose, and left for you: the `origin` remote still points at
`github.com/novis-lang/mwl.git`, and the checkout is still `<repo>`. Renaming either from inside a
script running out of that checkout is how you lose an afternoon; `--apply` prints both as a
closing note instead.
"""

import argparse
import os
import re
import subprocess
import time
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

OLD = "mwl"
NEW = "nvs"

#: Whole strings, replaced before either token rule and only outside DATA_FILES. Order matters:
#: longest first, so the Cargo description does not first become the title and then read
#: "...programming language: a ... language for web and CLI".
LITERALS = [
    (
        "MWL — Modern Web Lang: a JIT-compiled, memory-safe language for web and CLI",
        "Novis — the web-native programming language: JIT-compiled, memory-safe, for web and CLI",
    ),
    ("MWL — Modern Web Lang", "Novis — The Web-Native Programming Language"),
    ("MWL - Modern Web Lang", "Novis - The Web-Native Programming Language"),
    ("Modern Web Lang", "The Web-Native Programming Language"),
    # `MWL'S OWN LICENSE` is a heading `mwl info` prints in caps, and one assertion on it.
    # The prose rule would make it `Novis'S`.
    ("MWL'S", "NOVIS'S"),
]

#: Prose: the all-caps spelling, standing as its own word. The look-behind is identifier
#: characters only, so `MWL_JOBS`, `MWLC` and `MWL1` are left to the token rule, while `MWL.`
#: at the end of a sentence, `MWL-native`, `MWL/PHP` and `MWL's` are all prose. No all-caps
#: `MWL` is adjacent to `-`, `/` or `.` in a path anywhere in the tree -- paths are lowercase.
#:
#: `<?` is the one exception, and it cost a red test to find. `<?MWL` is a *mis-cased open tag*:
#: it appears in a lexer test, a diagnostic's doc comment and two ADRs, always as the example of
#: the casing `E_RESERVED_SPELLING_CASE` rejects. Neither neighbour is an identifier character,
#: so the plain word boundary read it as prose and produced `<?Novis`, which is seven characters
#: where the lexer matches five and so lexes as inline HTML rather than a tag. It is syntax, not
#: a name: it takes the token rule and becomes `<?NVS`, still mis-cased, still the point.
PROSE = re.compile(r"(?<![A-Za-z0-9_])(?<!<\?)MWL(?![A-Za-z0-9_])")

#: The token rule, any capitalisation.
TOKEN = re.compile(OLD, re.IGNORECASE)

#: Files where the project name is *data* fed to a case-mapping member and asserted back, not
#: prose. They take the token rule alone. Each was read in full: none of them contains a prose
#: `MWL` as well, so denying the whole file costs nothing. Paths are pre-rename.
DATA_FILES = {
    # `upper("mwl")` -> `MWL`, `lower("MWL")` -> `mwl`, `upperFirst("mwl runs")` -> `Mwl runs`
    "tests/conformance/core/str-case-members.mwlt",
    # `lowerFirst("MWL")` -> `mWL`
    "tests/conformance/core/str-first-letter-case-members.mwlt",
    "tests/differential/core/str-upper-first-diverges-from-ucfirst.mwlt",
    # `upper("mwl")` -> `MWL`, alone on its line in `--EXPECT--`. The differential sibling of
    # `str-case-members`, and the one this list was missing on the first run: a bare `MWL` in an
    # expectation block is indistinguishable from prose by shape, and only the oracle caught it.
    "tests/differential/core/str-upper-diverges-from-strtoupper.mwlt",
    # `new Slug("MWL")` -> `mwl`
    "tests/conformance/class/a-constructor-reaches-a-private-method.mwlt",
    # `assert_eq!(output_of(source), "MWL|Runs|abab")` over `upper("mwl")`
    "crates/mwl-codegen/tests/core_str.rs",
}

#: Never walked. `target` is 8.8G of build output that cargo will rebuild under the new crate
#: names anyway; `php-src` is a vendored upstream tree that is not ours to rename; `.loop` is
#: 16M of append-only session logs, a historical record of runs that really were called MWL.
EXCLUDE_DIRS = {".git", "target", "php-src", "__pycache__", "node_modules", ".loop"}

#: Untracked trees that are working state rather than build output, and so are in scope.
UNTRACKED_SCOPE = {"examples", ".agent-tmp"}

#: ...except the captured output under them. `verify.py` writes `.agent-tmp/verify-test.log`, and
#: every cargo line in it quotes an absolute path under a checkout still called `<repo>` -- so
#: the audit would report a fresh finding after every run, for a file the next run overwrites.
#: It is output, like `target`. Rewriting it would also falsify a record of what actually ran.
SKIP_SUFFIXES = (".log",)

#: This file, excluded from its own scope. It is the one place in the tree where `mwl` and `MWL`
#: are *the subject* rather than a name -- the regexes, DATA_FILES and the survey above all quote
#: the old spelling on purpose -- so rewriting it would destroy the record and, on a second run,
#: the tool. It stays as written; the audit skips it for the same reason.
SELF = "tools/rename-project.py"

MAX_BYTES = 4 * 1024 * 1024


def git(*args, check=True):
    out = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8"
    )
    if check and out.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed:\n{out.stderr}")
    return out


def tracked_paths():
    out = git("ls-files", "-z").stdout
    return {p for p in out.split("\0") if p}


def carry_case(match):
    """`mwl` -> `nvs`, `MWL` -> `NVS`, `Mwl` -> `Nvs`, `mWL` -> `nVS`.

    Per character, not per word: the three letters map positionally, so any capitalisation --
    including the `mWL` that `lowerFirst` produces -- comes out right without being enumerated.
    """
    return "".join(
        new.upper() if old.isupper() else new for old, new in zip(match.group(0), NEW)
    )


def rewrite(text, prose):
    """The whole substitution, in the one order that is correct.

    Literals first (they contain `MWL`, so a token pass would eat them), prose second (it
    consumes the standalone all-caps spelling), tokens last over whatever is left.
    """
    if prose:
        for old, new in LITERALS:
            text = text.replace(old, new)
        text = PROSE.sub("Novis", text)
    return TOKEN.sub(carry_case, text)


def rename(name):
    """A path component. Mechanical only -- paths are lowercase and never prose."""
    return TOKEN.sub(carry_case, name)


def walk(tracked_only):
    """Every file in scope, as a path relative to ROOT, posix-separated."""
    keep = tracked_paths() - {SELF}
    if tracked_only:
        return sorted(keep)
    found = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        rel = Path(dirpath).relative_to(ROOT)
        dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDE_DIRS)
        for name in sorted(filenames):
            p = (rel / name).as_posix()
            top = p.split("/", 1)[0]
            if p == SELF:
                continue
            if p not in keep and p.endswith(SKIP_SUFFIXES):
                continue
            if p in keep or top in UNTRACKED_SCOPE or Path(dirpath) == ROOT:
                found.append(p)
    return sorted(set(found) | keep)


def read(path):
    """`(text, note)`. `text` is None when the file must not be rewritten."""
    full = ROOT / path
    try:
        raw = full.read_bytes()
    except OSError as exc:
        return None, f"unreadable ({exc.strerror})"
    if len(raw) > MAX_BYTES:
        return None, f"skipped, {len(raw) // 1024}K over the {MAX_BYTES // 1024 // 1024}M cap"
    if b"\0" in raw:
        return None, "binary"
    try:
        return raw.decode("utf-8"), None
    except UnicodeDecodeError:
        return None, "not UTF-8"


def content_pass(paths, apply):
    """Rewrite file contents in place. Bytes in, bytes out -- no newline translation, so the
    CRLF fixtures under `**/fixtures/crlf/**` and the `eol=lf` guarantee both survive."""
    changed, skipped, counts = [], [], {"literal": 0, "prose": 0, "token": 0}
    for path in paths:
        text, note = read(path)
        if text is None:
            if note != "binary" or OLD in path.lower():
                skipped.append((path, note))
            continue
        prose = path not in DATA_FILES
        new = rewrite(text, prose)
        if new == text:
            continue
        rest = text
        if prose:
            for old, _ in LITERALS:
                counts["literal"] += rest.count(old)
                rest = rest.replace(old, "")
            counts["prose"] += len(PROSE.findall(rest))
            rest = PROSE.sub("", rest)
        counts["token"] += len(TOKEN.findall(rest))
        changed.append(path)
        if apply:
            (ROOT / path).write_bytes(new.encode("utf-8"))
    return changed, skipped, counts


def path_renames(tracked_only):
    """Directories deepest-first, then files. Directories move whole subtrees, so the file pass
    runs over the post-move tree and only ever touches basenames."""
    dirs = []
    for dirpath, dirnames, _ in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        rel = Path(dirpath).relative_to(ROOT)
        for d in dirnames:
            if OLD in d.lower():
                dirs.append((rel / d).as_posix())
    dirs.sort(key=lambda p: p.count("/"), reverse=True)
    files = [p for p in walk(tracked_only) if OLD in Path(p).name.lower()]
    return dirs, files


def move(rel_old, rel_new, tracked):
    """`git mv` when git knows the path, so the rename is staged rather than inferred.

    Retried, because on Windows a directory rename issued moments after the content pass wrote
    into it loses to whatever still holds a handle -- an indexer, a watcher, an antivirus scan --
    with `WinError 5`. The handle is released within a moment; the first run of this tool hit it
    on exactly one of the ten crate directories and the same `git mv` then succeeded by hand.
    """
    last = None
    for attempt in range(6):
        if attempt:
            time.sleep(0.25 * attempt)
        if tracked:
            out = git("mv", rel_old, rel_new, check=False)
            if out.returncode == 0:
                return
            last = out.stderr.strip()
        try:
            (ROOT / rel_new).parent.mkdir(parents=True, exist_ok=True)
            os.rename(ROOT / rel_old, ROOT / rel_new)
            return
        except OSError as exc:
            last = exc
    raise SystemExit(f"could not rename {rel_old} -> {rel_new}: {last}")


def rename_pass(tracked_only, apply):
    """`(dir_renames, file_renames)`, each a list of `(old, new)` relative posix paths."""
    keep = tracked_paths()
    dirs, files = path_renames(tracked_only)

    dir_renames = []
    for d in dirs:
        new = (Path(d).parent / rename(Path(d).name)).as_posix()
        dir_renames.append((d, new))
        if apply:
            move(d, new, any(p == d or p.startswith(d + "/") for p in keep))

    if apply:
        # The tree moved beneath us; re-derive so the file pass sees post-move paths.
        keep = tracked_paths()
        _, files = path_renames(tracked_only)
    else:
        # Same thing, simulated: apply the deepest matching directory move to each path.
        moved = []
        for p in files:
            for old, new in dir_renames:
                if p == old or p.startswith(old + "/"):
                    p = new + p[len(old) :]
                    break
            moved.append(p)
        files = moved

    file_renames = []
    for f in sorted(set(files)):
        new = (Path(f).parent / rename(Path(f).name)).as_posix()
        file_renames.append((f, new))
        if apply:
            move(f, new, f in keep)
    return dir_renames, file_renames


def audit(tracked_only):
    """What, if anything, is still called `mwl`."""
    bad_paths, bad_files = [], []
    for path in walk(tracked_only):
        if OLD in path.lower():
            bad_paths.append(path)
        text, _ = read(path)
        if text is None:
            continue
        hits = len(TOKEN.findall(text)) + text.count("Modern Web Lang")
        if hits:
            bad_files.append((path, hits))
    return bad_paths, bad_files


def report(paths, changed, skipped, counts, dir_renames, file_renames, tracked_only):
    print(f"scope             {len(paths)} files" + (" (tracked only)" if tracked_only else ""))
    print(f"content rewrites  {len(changed)} files")
    print(f"  literals        {counts['literal']:>6}  MWL — Modern Web Lang -> Novis — ...")
    print(f"  prose  MWL      {counts['prose']:>6}  -> Novis")
    print(f"  token  mwl      {counts['token']:>6}  -> nvs, case carried per character")
    print(f"path renames      {len(dir_renames)} directories, {len(file_renames)} files")
    for old, new in dir_renames:
        print(f"  {old}  ->  {new}")
    by_ext = {}
    for old, _ in file_renames:
        by_ext[Path(old).suffix or "(none)"] = by_ext.get(Path(old).suffix or "(none)", 0) + 1
    for ext, n in sorted(by_ext.items(), key=lambda kv: -kv[1]):
        print(f"  {n:>5} {ext}")
    print(f"prose rule held off in {len(DATA_FILES)} case-mapping fixtures")
    if skipped:
        print(f"not rewritten ({len(skipped)}):")
        for path, note in skipped:
            print(f"  {path}: {note}")


def main():
    ap = argparse.ArgumentParser(
        description="Rename MWL -> Novis and mwl -> nvs across the whole tree.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument("--apply", action="store_true", help="write the changes (default: dry run)")
    ap.add_argument("--audit", action="store_true", help="only report what is still named mwl")
    ap.add_argument("--tracked-only", action="store_true", help="skip examples/ and .agent-tmp/")
    ap.add_argument("--no-fmt", action="store_true", help="skip `cargo fmt --all` after --apply")
    opts = ap.parse_args()

    if opts.audit:
        bad_paths, bad_files = audit(opts.tracked_only)
        for p in bad_paths:
            print(f"path    {p}")
        for p, n in bad_files:
            print(f"content {p}  ({n})")
        if bad_paths or bad_files:
            print(f"\nstill named mwl: {len(bad_paths)} paths, {len(bad_files)} files")
            return 1
        print("clean: no `mwl` in any path or file in scope")
        return 0

    paths = walk(opts.tracked_only)
    changed, skipped, counts = content_pass(paths, opts.apply)
    dir_renames, file_renames = rename_pass(opts.tracked_only, opts.apply)
    report(paths, changed, skipped, counts, dir_renames, file_renames, opts.tracked_only)

    if not opts.apply:
        print("\ndry run -- nothing written. Re-run with --apply.")
        return 0

    if not opts.no_fmt:
        print("\ncargo fmt --all ...")
        subprocess.run(["cargo", "fmt", "--all"], cwd=ROOT)

    print("\naudit:")
    bad_paths, bad_files = audit(opts.tracked_only)
    for p in bad_paths:
        print(f"  path    {p}")
    for p, n in bad_files:
        print(f"  content {p}  ({n})")
    if bad_paths or bad_files:
        print("  FAILED -- see above")
        return 1
    print("  clean: no `mwl` in any path or file in scope")
    print("\nLeft for you, deliberately:")
    print("  git remote set-url origin https://github.com/novis-lang/novis.git   (rename it there first)")
    print("  the checkout is still <repo> -- rename it from outside this directory")
    print("\nNext: python tools/verify.py")
    return 0


if __name__ == "__main__":
    sys.exit(main())
