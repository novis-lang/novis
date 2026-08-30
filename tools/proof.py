#!/usr/bin/env python3
"""Prove `docs/novis.md` is enough: a model that has only that file writes programs, and the binary judges them.

    python tools/proof.py --prepare            # a fresh run directory holding novis.md, TASKS.md and the prompt
    python tools/proof.py --run [--model M]    # --prepare, then drive `claude -p` blind in that directory, then judge
    python tools/proof.py --judge [DIR]        # run every task's program the model wrote and score it
    python tools/proof.py --list               # the tasks, one line each

## What this proves

`docs/novis.md` exists so that something which has never seen this repository -- a search engine,
a language model, a person -- can write correct Novis from one file. The only honest test of that
is to hand the file to such a reader and run what it writes. So:

1. `--prepare` copies `docs/novis.md` into an empty directory under `.agent-tmp/proof/` beside a
   `TASKS.md` listing ordinary programming tasks (a FizzBuzz, a class hierarchy, a JSON round
   trip, a multi-file program, tests, ...) and a `PROMPT.md` telling the reader to solve them from
   the reference alone, **without running anything** -- the point is what the document conveys,
   not what a compiler's diagnostics teach on the third try.
2. The reader writes `tasks/<id>/` with its program files and a `RUN` line -- the command a user
   would type -- plus `NOTES.md` saying what the reference left unclear.
3. `--judge` runs every `RUN` line through the real binary, in that task's directory, and compares
   what came out with what the task asked for: a checklist of lines the output must contain, or the
   exact text, and the exit status. It writes `report.md` next to the tasks, with the diagnostic or
   the diff for every failure, and exits non-zero when any task failed.

A failure is a finding about the *reference*: the report says which task, which spelling the
reader chose and what the compiler said, which is exactly the sentence the chapter was missing.
Fix the chapter under `docs/reference/`, regenerate, rerun.

## Driving the reader

`--run` uses the `claude` CLI in print mode, with the run directory as its working directory and
only file tools allowed, so the reader has the reference and nothing else. Without the CLI, or to
use another model, `--prepare` prints the prompt path: hand `PROMPT.md` to any agent whose working
directory is the run directory, then `--judge` it. The tasks live in this file so a rerun months
from now asks the same questions of a newer reference and a newer reader.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REFERENCE = ROOT / "docs" / "novis.md"
RUNS = ROOT / ".agent-tmp" / "proof"
BINARY = ROOT / "target" / "debug" / ("nvs.exe" if os.name == "nt" else "nvs")
TIMEOUT = 120


@dataclass
class Task:
    id: str
    title: str
    prompt: str
    #: Lines that must each appear somewhere in stdout.
    contains: list[str] = field(default_factory=list)
    #: The exact stdout, when the task fixes it entirely.
    exact: str | None = None
    exit_code: int = 0


TASKS: list[Task] = [
    Task(
        "hello", "Hello",
        "Write a program that prints exactly `Hello, Novis!` followed by a newline.",
        exact="Hello, Novis!",
    ),
    Task(
        "fizzbuzz", "FizzBuzz",
        "Print the numbers 1 to 15, one per line, replacing multiples of 3 with `Fizz`, multiples of "
        "5 with `Buzz`, and multiples of both with `FizzBuzz`. Put the logic in a static method that "
        "takes the number and returns the string to print.",
        exact="1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz",
    ),
    Task(
        "wordcount", "Word frequencies",
        "Given the text `the quick brown fox jumps over the lazy dog the fox` (a string constant in "
        "the program), count how often each word occurs, case-insensitively, and print one line per "
        "word as `word=count`, most frequent first and alphabetical among equal counts. Use the "
        "`Core` array and string members rather than hand-rolled loops where one exists.",
        exact="the=3\nfox=2\nbrown=1\ndog=1\njumps=1\nlazy=1\nover=1\nquick=1",
    ),
    Task(
        "shapes", "Interfaces and classes",
        "Declare an interface `Shape` with `area(): float` and `name(): string`, and two classes "
        "`Circle(float $radius)` and `Rect(float $w, float $h)` implementing it. Build an array of "
        "three shapes -- a circle of radius 1, a 2x3 rectangle, a 1x1 rectangle -- and print one line "
        "per shape as `name: area` with the area rounded to two decimals (e.g. `circle: 3.14`), then "
        "a final line `total: <sum>` with the same rounding. Use `Core\\Math::PI`.",
        exact="circle: 3.14\nrect: 6.00\nrect: 1.00\ntotal: 10.14",
    ),
    Task(
        "json", "JSON round trip",
        "Declare a class `User` with a `string $name`, an `int $age` and a `bool $admin` that can be "
        "encoded to JSON and decoded back with the derived codec. Encode a list of two users -- "
        "`ada` (36, admin) and `bob` (7, not admin) -- print the JSON on one line, decode that JSON "
        "back into users, and print `decoded: <count> users, first=<name>`.",
        contains=['[{"name":"ada","age":36,"admin":true},{"name":"bob","age":7,"admin":false}]',
                  "decoded: 2 users, first=ada"],
    ),
    Task(
        "errors", "Exceptions",
        "Declare a custom throwable `ValidationError` that carries a `string $field` besides its "
        "message. Write a static method `check(string $email)` that throws it with field `email` when "
        "the argument does not contain `@`, and returns the lower-cased email otherwise. Call it with "
        "`Ada@Example.Test` and then with `nobody`, printing `ok: <email>` for a success and "
        "`invalid <field>: <message>` for the failure, and print `done` from a `finally` block after "
        "each call, so the output has four lines.",
        contains=["ok: ada@example.test", "done", "invalid email:"],
    ),
    Task(
        "multifile", "A program spread over files",
        "Write a program of three files: `main.nvs` (the entry), `src/Inventory/Item.nvs` declaring "
        "`App\\Inventory\\Item` (a `string $sku`, an `int $qty`, a `decimal $price`) and "
        "`src/Inventory/Stock.nvs` declaring `App\\Inventory\\Stock` which holds items, can `add` one, "
        "and answers `total(): decimal` (sum of `qty * price`). The entry reaches the classes through "
        "an `autoload` declaration, never `require`, adds three items -- `A1` 2 x 9.99, `B2` 1 x 0.01, "
        "`C3` 10 x 1.50 -- and prints `items=3` and `total=34.99` (two decimals).",
        exact="items=3\ntotal=34.99",
    ),
    Task(
        "generator", "A generator",
        "Write a generator method that yields the Fibonacci numbers lazily (1, 1, 2, 3, 5, ...) and a "
        "loop that prints the first ten of them on one line separated by spaces, stopping the "
        "generator with `break`.",
        exact="1 1 2 3 5 8 13 21 34 55",
    ),
    Task(
        "dates", "Dates",
        "Parse `2024-01-31` as a date in UTC, print it 45 days later as `yyyy-MM-dd`, print the "
        "weekday name of that day, and print the number of days between the two dates. Three lines: "
        "`later=…`, `weekday=…`, `days=45`.",
        contains=["later=2024-03-16", "days=45"],
    ),
    Task(
        "tests", "Tests",
        "Write a `Stack` class (push, pop, count; pop on an empty stack throws `LogicError`) and "
        "three tests for it using the language's built-in test attribute and assertions: pushing "
        "then popping returns the pushed value, count follows pushes and pops, and popping an "
        "empty stack throws. The `RUN` line must run the tests, not the program.",
        contains=["3 passed"],
    ),
    Task(
        "csv", "A CSV report",
        "Parse this CSV text (a string constant) with its header row: `category,amount` then rows "
        "`food,12.50`, `rent,800`, `food,7.25`, `fun,30`, `rent,20`. Sum the amounts per category and "
        "print one line per category as `category: total` with two decimals, in descending order of "
        "total.",
        exact="rent: 820.00\nfun: 30.00\nfood: 19.75",
    ),
    Task(
        "regex", "Regular expressions",
        "From the text `contact ada@example.test or BOB@example.org, not bob@example.org again` "
        "extract every email address, lower-case them, drop duplicates, sort them, and print one per "
        "line.",
        exact="ada@example.test\nbob@example.org",
    ),
    Task(
        "tasks", "Concurrency",
        "Square the numbers 1 to 8 concurrently with at most two running at a time using the "
        "structured-concurrency member for mapping over an array, then print `sum=204`. Then run two "
        "named tasks at once -- one returning the string `left`, one the int `42` -- and print "
        "`left 42` from the typed result.",
        exact="sum=204\nleft 42",
    ),
    Task(
        "enums", "Enums and match",
        "Declare an enum `Status` with cases `Draft`, `Review`, `Published`. Write a static method "
        "`next(Status $s): Status` that advances Draft to Review and Review to Published, and throws "
        "`LogicError` for Published. Starting from Draft, advance until it throws, printing each "
        "state's integer value and name-like label on one line each (`0 draft`, `1 review`, "
        "`2 published`) using a `match`, then print the caught message.",
        contains=["0 draft", "1 review", "2 published"],
    ),
    Task(
        "template", "Inline HTML",
        "Write a file that mixes HTML and code: a list of three product names -- `pen`, `ink`, "
        "`paper` -- rendered as `<li>` items inside a `<ul>`, using the short echo tag for each name, "
        "and a heading `<h1>3 products</h1>` above the list with the count computed in code.",
        contains=["<h1>3 products</h1>", "<li>pen</li>", "<li>ink</li>", "<li>paper</li>"],
    ),
]

PROMPT = """\
You are a programmer meeting a new language, Novis, for the first time. The file `novis.md` in this
directory is its complete reference. **It is the only thing you may read** — do not look at any
other file on this machine, do not search the web, and do not guess from PHP: where Novis differs
from PHP, the reference says so, and PHP spellings are refused by the compiler.

Solve every task in `TASKS.md`. For each task:

1. Create the directory `tasks/<id>/` and write the program file(s) there. Unless the task names the
   files, the entry file is `main.nvs`.
2. Write `tasks/<id>/RUN` holding the one command line a user runs from inside that directory to
   execute the solution — normally `nvs run main.nvs`; for the tests task, the command that runs
   tests.
3. Print exactly what the task asks for, byte for byte, and nothing else.

You have no compiler: do not try to run anything. Work from the reference — search it for the
anchors and keywords its *How to read this file* section describes, read the chapter that owns each
construct, and copy the spelling of the examples. Use the `Core` library's members (Part B) rather
than reimplementing what they do; every member's signature and description is there.

When every task is written, write `NOTES.md` at the top level: for each task, one or two lines on
what the reference made easy, and what you had to guess at or could not find — that feedback is the
whole point of the exercise, so be specific (name the section and the sentence you wanted).
"""


def tasks_md() -> str:
    lines = ["# Tasks", "", "Solve each in `tasks/<id>/` with a `RUN` file. Print exactly what is asked.", ""]
    for n, t in enumerate(TASKS, 1):
        lines.append(f"## {n}. `{t.id}` — {t.title}")
        lines.append("")
        lines.append(t.prompt)
        lines.append("")
    return "\n".join(lines)


def prepare(run_dir: Path | None) -> Path:
    if not REFERENCE.is_file():
        sys.exit("proof.py: docs/novis.md does not exist -- `python tools/reference.py` first")
    run_dir = run_dir or RUNS / dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    if run_dir.exists():
        shutil.rmtree(run_dir)
    run_dir.mkdir(parents=True)
    shutil.copy(REFERENCE, run_dir / "novis.md")
    (run_dir / "TASKS.md").write_text(tasks_md(), encoding="utf-8", newline="\n")
    (run_dir / "PROMPT.md").write_text(PROMPT, encoding="utf-8", newline="\n")
    print(f"proof.py: prepared {run_dir.relative_to(ROOT).as_posix()}")
    print(f"  hand PROMPT.md to a reader whose working directory is that one, then:")
    print(f"  python tools/proof.py --judge {run_dir.relative_to(ROOT).as_posix()}")
    return run_dir


def drive(run_dir: Path, model: str | None) -> None:
    exe = shutil.which("claude") or shutil.which("claude.cmd")
    if not exe:
        sys.exit("proof.py: no `claude` CLI on PATH -- use --prepare and drive the reader by hand")
    cmd = [exe, "-p", PROMPT, "--allowedTools", "Read", "Write", "Edit", "Glob", "Grep",
           "--permission-mode", "acceptEdits", "--output-format", "text"]
    if model:
        cmd += ["--model", model]
    print(f"proof.py: driving `claude -p` in {run_dir.relative_to(ROOT).as_posix()} ...")
    p = subprocess.run(cmd, cwd=run_dir, capture_output=True, encoding="utf-8", errors="replace")
    (run_dir / "reader.log").write_text((p.stdout or "") + (p.stderr or ""), encoding="utf-8",
                                        newline="\n")
    if p.returncode != 0:
        print(f"proof.py: the reader exited {p.returncode}; see reader.log")


def normalize(s: str) -> str:
    return "\n".join(line.rstrip() for line in s.replace("\r\n", "\n").split("\n")).strip("\n")


def judge_one(task: Task, run_dir: Path) -> tuple[bool, str]:
    where = run_dir / "tasks" / task.id
    if not where.is_dir():
        return False, "no `tasks/%s/` directory was written" % task.id
    run_file = where / "RUN"
    if not run_file.is_file():
        return False, "no `RUN` file"
    line = run_file.read_text(encoding="utf-8").strip().splitlines()
    line = line[0].strip() if line else ""
    argv = line.split()
    if not argv or argv[0] not in ("nvs", "nvs.exe"):
        return False, f"RUN is `{line}`, which does not start with `nvs`"
    argv[0] = str(BINARY)
    try:
        p = subprocess.run(argv, cwd=where, capture_output=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return False, f"`{line}` timed out after {TIMEOUT}s"
    stdout = p.stdout.decode("utf-8", "replace")
    stderr = p.stderr.decode("utf-8", "replace")
    detail = f"`{line}` exited {p.returncode}\n"
    if p.returncode != task.exit_code:
        return False, detail + "--- stdout\n" + normalize(stdout) + "\n--- stderr\n" + normalize(stderr)
    if task.exact is not None and normalize(stdout) != normalize(task.exact):
        return False, detail + "--- expected\n" + normalize(task.exact) + "\n--- got\n" + normalize(stdout)
    missing = [c for c in task.contains if c not in stdout]
    if missing:
        return False, detail + f"missing {missing!r}\n--- got\n" + normalize(stdout)
    return True, detail.strip()


def judge(run_dir: Path) -> int:
    if not BINARY.is_file():
        sys.exit(f"proof.py: no binary at {BINARY.relative_to(ROOT).as_posix()}")
    results = []
    for task in TASKS:
        ok, detail = judge_one(task, run_dir)
        results.append((task, ok, detail))
        print(f"  {'ok  ' if ok else 'FAIL'}  {task.id:<12} {task.title}")
    passed = sum(1 for _, ok, _ in results if ok)
    lines = [f"# Proof report — {run_dir.name}", "",
             f"**{passed} of {len(TASKS)} tasks pass** against `{BINARY.relative_to(ROOT).as_posix()}`.", ""]
    for task, ok, detail in results:
        lines.append(f"## {'PASS' if ok else 'FAIL'} — `{task.id}` {task.title}")
        lines.append("")
        lines.append("```")
        lines.append(detail)
        lines.append("```")
        lines.append("")
    notes = run_dir / "NOTES.md"
    if notes.is_file():
        lines.append("## The reader's notes")
        lines.append("")
        lines.append(notes.read_text(encoding="utf-8"))
    (run_dir / "report.md").write_text("\n".join(lines), encoding="utf-8", newline="\n")
    print(f"proof.py: {passed} of {len(TASKS)} tasks pass -- report in "
          f"{(run_dir / 'report.md').relative_to(ROOT).as_posix()}")
    return 0 if passed == len(TASKS) else 1


def latest_run() -> Path:
    runs = sorted(p for p in RUNS.glob("*") if p.is_dir()) if RUNS.is_dir() else []
    if not runs:
        sys.exit("proof.py: no run directory under .agent-tmp/proof/ -- --prepare first")
    return runs[-1]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--prepare", action="store_true", help="create a fresh run directory")
    ap.add_argument("--run", action="store_true", help="prepare, drive `claude -p`, judge")
    ap.add_argument("--judge", nargs="?", const="", metavar="DIR",
                    help="judge a run directory (default: the latest)")
    ap.add_argument("--list", action="store_true", help="print the tasks")
    ap.add_argument("--model", help="with --run: the model to drive")
    ap.add_argument("--dir", help="with --prepare/--run: use this run directory")
    opts = ap.parse_args()
    if opts.list:
        for t in TASKS:
            print(f"{t.id:<12} {t.title}")
        return 0
    run_dir = (ROOT / opts.dir) if opts.dir else None
    if opts.run:
        run_dir = prepare(run_dir)
        drive(run_dir, opts.model)
        return judge(run_dir)
    if opts.prepare:
        prepare(run_dir)
        return 0
    if opts.judge is not None:
        return judge((ROOT / opts.judge) if opts.judge else latest_run())
    ap.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
