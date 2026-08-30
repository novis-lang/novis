---
summary: a compiled pattern carrying its flags, taken by every `Core\Regex` member in place of a pattern string
keywords: /i, /m, /s, /U, pattern modifiers, compiled regex, case-insensitive, multiline, dotall, ungreedy
---

A `Core\Regex\Pattern` is a compiled pattern with its four flags — `caseInsensitive`, `multiline`,
`dotAll`, `ungreedy` — answered by `Core\Regex::compile` and accepted wherever a `Core\Regex` member
takes a pattern. It has no members of its own: compile once where a pattern is reused or where a
flag is needed, and pass the handle. A pattern neither engine can compile throws from `compile`
itself.

```nvs
<?nvs
Core\Regex\Pattern $tag = Core\Regex::compile("<.+>", {ungreedy: true});
echo Core\Regex::match("<a><b>", $tag)?->text() ?? "none", "\n";
Core\Regex\Pattern $vowel = Core\Regex::compile("[aeiou]", {caseInsensitive: true});
echo Core\Regex::replace("Banana", $vowel, "."), " ", Core\Arr::count(Core\Regex::matchAll("Banana", $vowel)), "\n";
string $bad = "(unclosed";
try {
    Core\Regex\Pattern $never = Core\Regex::compile($bad);
    echo "compiled\n";
} catch (Throwable $e) {
    echo "refused\n";
}
```
```output
<a>
B.n.n. 3
refused
```
