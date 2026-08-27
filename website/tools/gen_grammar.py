"""Generate the Novis TextMate grammar from the compiler's keyword table.

The keyword list has exactly one home: `Keyword::from_lowercase` in
`crates/nvs-syntax/src/token.rs`. This tool reads it and emits a Shiki-loadable
grammar, so a keyword added to the lexer highlights on this site the same day
without anyone remembering to come here.

Only the *keywords* are derived. The rest of the grammar — strings, comments,
attributes, duration literals, variables, operators — is written below by hand,
because none of it is a list the compiler holds in one enumerable place. If that
changes (a token table that names its operators), derive those too.

What happens when the lexer gains a keyword this file has no category for: it
lands in `keyword.other.nvs`, still highlighted, and the tool prints it so
someone can classify it. It never fails the build over a cosmetic question, and
it never silently drops a keyword.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from common import REPO, WEB, info, ok, warn

TOKEN_RS = REPO / "crates" / "nvs-syntax" / "src" / "token.rs"
OUT = WEB / "src" / "grammars" / "nvs.tmLanguage.json"

# Which scope each keyword gets. Scope names are the standard TextMate ones, so
# every Shiki theme colours them without a per-theme mapping here.
CATEGORY: dict[str, str] = {}


def _cat(scope: str, *words: str) -> None:
    for w in words:
        CATEGORY[w] = scope


_cat(
    "keyword.control.nvs",
    "break", "case", "catch", "continue", "default", "do", "else", "elseif",
    "finally", "for", "foreach", "goto", "if", "match", "return", "switch",
    "throw", "try", "while", "yield",
)
_cat(
    "keyword.declaration.nvs",
    "abstract", "autoload", "class", "const", "declare", "enum", "extends",
    "final", "fn", "function", "implements", "insteadof", "interface",
    "namespace", "trait", "use",
)
_cat(
    "storage.modifier.nvs",
    "global", "inout", "lateinit", "private", "protected", "public", "readonly",
    "secret", "static", "tainted", "var",
)
_cat(
    "support.type.nvs",
    "array", "bool", "bytes", "callable", "decimal", "float", "int", "iterable",
    "mixed", "never", "object", "string", "uint", "void",
)
_cat("constant.language.nvs", "false", "null", "true")
_cat("variable.language.nvs", "parent", "self")
_cat(
    "keyword.operator.word.nvs",
    "and", "as", "clone", "instanceof", "new", "or", "xor",
)
_cat(
    "keyword.other.nvs",
    "die", "echo", "empty", "eval", "exit", "extract", "include", "include_once",
    "isset", "list", "print", "require", "require_once", "settype", "unset",
)

# Order matters: longer alternatives first so `require_once` is not eaten by
# `require`. Applied per bucket at render time.
KEYWORD_RE = re.compile(r'^\s*"([a-z_0-9]+)"\s*=>\s*Self::', re.MULTILINE)


def read_keywords() -> list[str]:
    if not TOKEN_RS.is_file():
        raise SystemExit(
            f"cannot read {TOKEN_RS}.\n"
            "The grammar is generated from the compiler's keyword table; without\n"
            "the Rust workspace present there is nothing to generate it from."
        )
    text = TOKEN_RS.read_text(encoding="utf-8")
    words = sorted(set(KEYWORD_RE.findall(text)))
    if len(words) < 40:
        raise SystemExit(
            f"only {len(words)} keywords matched in {TOKEN_RS.name}.\n"
            "That is far below the real count, so the table's shape has changed and\n"
            "KEYWORD_RE in this file no longer matches it. Fix the pattern rather\n"
            "than shipping a grammar that highlights a tenth of the language."
        )
    return words


def _alt(words: list[str]) -> str:
    """A regex alternation, longest first so prefixes do not shadow."""
    return "|".join(sorted(words, key=len, reverse=True))


def build(words: list[str]) -> tuple[dict, list[str]]:
    buckets: dict[str, list[str]] = {}
    unknown: list[str] = []
    for w in words:
        scope = CATEGORY.get(w)
        if scope is None:
            unknown.append(w)
            scope = "keyword.other.nvs"
        buckets.setdefault(scope, []).append(w)

    # The lookbehind keeps `$class`, `Foo\array` and `my_int` from matching a
    # keyword; the trailing `\b` keeps `interfaces` from matching `interface`.
    keyword_patterns = [
        {"name": scope, "match": r"(?<![$\\\w])(" + _alt(ws) + r")\b"}
        for scope, ws in sorted(buckets.items())
    ]

    grammar = {
        "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
        "name": "nvs",
        "displayName": "Novis",
        "aliases": ["novis"],
        "scopeName": "source.nvs",
        "fileTypes": ["nvs", "nvt"],
        "patterns": [
            {"include": "#open-tag"},
            {"include": "#attribute"},
            {"include": "#comment"},
            {"include": "#string"},
            {"include": "#duration"},
            {"include": "#number"},
            {"include": "#keyword"},
            {"include": "#declaration-name"},
            {"include": "#member"},
            {"include": "#call"},
            {"include": "#namespace"},
            {"include": "#variable"},
            {"include": "#operator"},
        ],
        "repository": {
            # `<?nvs` is the one open tag (ADR 0049). Anything before it is
            # inline output, not code.
            "open-tag": {
                "name": "punctuation.section.embedded.begin.nvs",
                "match": r"<\?nvs\b",
            },
            # Must precede #comment: `#[` starts an attribute, `#` a comment.
            "attribute": {
                "begin": r"#\[",
                "end": r"\]",
                "name": "meta.attribute.nvs",
                "beginCaptures": {"0": {"name": "punctuation.definition.attribute.nvs"}},
                "endCaptures": {"0": {"name": "punctuation.definition.attribute.nvs"}},
                "patterns": [
                    {"include": "#string"},
                    {"include": "#number"},
                    {"name": "entity.name.attribute.nvs", "match": r"[A-Za-z_][\w\\]*"},
                ],
            },
            "comment": {
                "patterns": [
                    {
                        "name": "comment.block.documentation.nvs",
                        "begin": r"/\*\*",
                        "end": r"\*/",
                    },
                    {"name": "comment.block.nvs", "begin": r"/\*", "end": r"\*/"},
                    {"name": "comment.line.double-slash.nvs", "match": r"//.*$"},
                    {"name": "comment.line.number-sign.nvs", "match": r"#(?!\[).*$"},
                ]
            },
            "string": {
                "patterns": [
                    {
                        "name": "string.quoted.double.nvs",
                        "begin": '"',
                        "end": '"',
                        "patterns": [
                            {"name": "constant.character.escape.nvs", "match": r"\\."},
                            {
                                "name": "variable.other.interpolated.nvs",
                                "match": r"\$\{?[A-Za-z_]\w*\}?",
                            },
                        ],
                    },
                    {
                        "name": "string.quoted.single.nvs",
                        "begin": "'",
                        "end": "'",
                        "patterns": [{"name": "constant.character.escape.nvs", "match": r"\\."}],
                    },
                ]
            },
            # Duration literals (ADR 0070): `250ms`, `10s`, `2h`.
            "duration": {
                "name": "constant.numeric.duration.nvs",
                "match": r"\b\d[\d_]*(?:\.\d[\d_]*)?(?:ns|us|ms|s|m|h|d|w)\b",
            },
            "number": {
                "patterns": [
                    {"name": "constant.numeric.hex.nvs", "match": r"\b0[xX][0-9a-fA-F_]+\b"},
                    {"name": "constant.numeric.binary.nvs", "match": r"\b0[bB][01_]+\b"},
                    {"name": "constant.numeric.octal.nvs", "match": r"\b0[oO][0-7_]+\b"},
                    {
                        "name": "constant.numeric.decimal.nvs",
                        "match": r"\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?\b",
                    },
                ]
            },
            "keyword": {"patterns": keyword_patterns},
            # `function name(`, `class Name`, `enum Name` — name the declared thing.
            "declaration-name": {
                "patterns": [
                    {
                        "match": r"\b(function|fn)\s+([A-Za-z_]\w*)",
                        "captures": {
                            "1": {"name": "keyword.declaration.nvs"},
                            "2": {"name": "entity.name.function.nvs"},
                        },
                    },
                    {
                        "match": r"\b(class|interface|enum|trait)\s+([A-Za-z_]\w*)",
                        "captures": {
                            "1": {"name": "keyword.declaration.nvs"},
                            "2": {"name": "entity.name.type.nvs"},
                        },
                    },
                ]
            },
            "member": {
                "patterns": [
                    {
                        "match": r"(->)\s*([A-Za-z_]\w*)\s*(?=\()",
                        "captures": {
                            "1": {"name": "keyword.operator.accessor.nvs"},
                            "2": {"name": "entity.name.function.member.nvs"},
                        },
                    },
                    {
                        "match": r"(->)\s*([A-Za-z_]\w*)",
                        "captures": {
                            "1": {"name": "keyword.operator.accessor.nvs"},
                            "2": {"name": "variable.other.property.nvs"},
                        },
                    },
                    {
                        "match": r"(::)\s*([A-Za-z_]\w*)\s*(?=\()",
                        "captures": {
                            "1": {"name": "keyword.operator.accessor.nvs"},
                            "2": {"name": "entity.name.function.static.nvs"},
                        },
                    },
                    {
                        "match": r"(::)\s*([A-Za-z_]\w*)",
                        "captures": {
                            "1": {"name": "keyword.operator.accessor.nvs"},
                            "2": {"name": "constant.other.class.nvs"},
                        },
                    },
                ]
            },
            "call": {
                "match": r"\b([A-Za-z_]\w*)\s*(?=\()",
                "name": "entity.name.function.call.nvs",
            },
            # `Core\Str`, `App\Models\User` — every built-in lives under `Core`.
            "namespace": {
                "patterns": [
                    {
                        "match": r"\b([A-Za-z_]\w*)(?=\\)",
                        "name": "entity.name.namespace.nvs",
                    },
                    {"match": r"\\", "name": "punctuation.separator.namespace.nvs"},
                    {"match": r"\b[A-Z]\w*\b", "name": "support.class.nvs"},
                ]
            },
            "variable": {"name": "variable.other.nvs", "match": r"\$[A-Za-z_]\w*"},
            "operator": {
                "name": "keyword.operator.nvs",
                "match": r"(?:\|>|\?\?=|<=>|\*\*=|\?->|\.\.\.|===|!==|<<=|>>=|\?\?|\|\||&&|==|!=|<=|>=|=>|->|\+\+|--|\+=|-=|\*=|/=|%=|\.=|\|=|&=|\^=|<<|>>|[-+*/%.!<>=&|^~?:])",
            },
        },
    }
    return grammar, unknown


def main(quiet: bool = False) -> dict:
    words = read_keywords()
    grammar, unknown = build(words)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(grammar, indent=2) + "\n", encoding="utf-8")
    if not quiet:
        ok(f"grammar: {len(words)} keywords from crates/nvs-syntax/src/token.rs")
        if unknown:
            warn(
                "keywords with no category in tools/gen_grammar.py (highlighted as "
                f"`keyword.other`): {', '.join(unknown)}"
            )
            info("add them to a _cat(...) call so they get the right colour")
    return grammar


if __name__ == "__main__":
    main()
