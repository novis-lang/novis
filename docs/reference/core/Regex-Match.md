---
summary: one match of a pattern — its text, its groups by number or name, and where it starts
keywords: regex match, matches, match offset, unmatched group, capture group, named group, match result
---

`Core\Regex::match` returns a `Core\Regex\Match`, or `null` when nothing matched. `Core\Regex::matchAll`
returns a list of them, and `Core\Regex::replaceWith` gives one to its callback. You never create one
yourself. `text()` returns the whole match. `group()` returns one group by number or by name. It
returns `null` for a group the pattern declares but this match did not use. `groups()` returns every
group in pattern order: a named group appears under its name, then under its number. `offset()` returns
where the match starts, counted in graphemes.

```nvs
<?nvs
var $m = Core\Regex::match('café au lait', '(?<drink>lait)');
if ($m != null) {
    echo $m->text(), ' ', $m->offset(), ' ', $m->group('drink') ?? '?', ' ', $m->group(1) ?? '?', "\n";
    foreach ($m->groups() as string $k => ?string $v) {
        echo $k, '=', $v ?? 'null', ';';
    }
    echo "\n";
}
var $opt = Core\Regex::match('ac', '(a)(b)?(c)');
echo $opt?->group(2) ?? 'no b', ' ', $opt?->group(3) ?? 'no c', "\n";
```
```output
lait 8 lait lait
0=lait;drink=lait;1=lait;
no b c
```
