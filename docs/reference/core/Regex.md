---
summary: regular expressions without delimiters — match, capture, replace and split, linear-time by default with a step budget for the patterns that need backtracking
keywords: preg_match, preg_match_all, preg_replace, preg_replace_callback, preg_split, preg_quote, PREG_OFFSET_CAPTURE, PREG_SET_ORDER, PREG_UNMATCHED_AS_NULL, pcre.backtrack_limit, regex, regexp, pattern, named group, lookahead, lookbehind, backreference, ReDoS
---

A `Core\Regex` pattern is a plain string — no `/…/` delimiters and no trailing modifiers; the
flags are options to `Core\Regex::compile`, which answers a `Core\Regex\Pattern` every other
member also takes in place of the string. A single-quoted string is the cheap spelling — `'\d+'`
has no escapes to double, where `"\\d+"` does. A pattern runs on a linear-time engine; one that needs
a lookaround or a backreference runs on a backtracking engine under a step budget, and exhausting
that budget throws rather than answering `false`. A pattern neither engine can compile throws too.
`match` answers `?Core\Regex\Match` — `null` when nothing matched — and `matchAll` a list of them;
in `replace`'s template `$1` and `${name}` are group references and `$$` a plain `$`, while
the value `replaceWith`'s callback returns is inserted as it is. Fixed text built into a pattern
goes through `quote`.

```nvs
<?nvs
string $log = "2026-08-30 GET /a 200; 2026-08-31 POST /b 404";
echo Core\Regex::matches($log, "\\b\\d{3}\\b") ? "has status" : "none", "\n";
var $m = Core\Regex::match($log, "(?<y>\\d{4})-(?<mo>\\d{2})-(?<d>\\d{2})");
echo $m?->text() ?? "none", " at ", $m?->offset() ?? -1, " day=", $m?->group("d") ?? "?", "\n";
foreach (Core\Regex::matchAll($log, "(GET|POST) (\\S+)") as Core\Regex\Match $hit) {
    echo $hit->group(1) ?? "?", " ", $hit->group(2) ?? "?", "\n";
}
echo Core\Regex::replace("2026-08-30", "(\\d+)-(\\d+)-(\\d+)", "$3/$2/$1"), "\n";
echo Core\Regex::replaceWith("a1b22", "\\d+", fn(Core\Regex\Match $x): string => "<" . $x->text() . ">"), "\n";
echo Core\Str::join(Core\Regex::split("a, b,c", ",\\s*"), "|"), "\n";
Core\Regex\Pattern $word = Core\Regex::compile("^[a-z]+$", {caseInsensitive: true});
echo Core\Regex::matches("Hello", $word) ? "word" : "not a word", "\n";
echo Core\Regex::quote("a.b*c?"), "\n";
```
```output
has status
2026-08-30 at 0 day=30
GET /a
POST /b
30/08/2026
a<1>b<22>
a|b|c
word
a\.b\*c\?
```
