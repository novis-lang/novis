---
summary: one match of a pattern — its text, its groups by number or name, and where it starts
keywords: preg_match $matches, PREG_OFFSET_CAPTURE, PREG_UNMATCHED_AS_NULL, capture group, named group, match result
---

A `Core\Regex\Match` is what `Core\Regex::match` answers (`null` when nothing matched), what
`Core\Regex::matchAll` lists, and what `Core\Regex::replaceWith` hands its callback; it is never
constructed by hand. `text()` is the whole match, `group()` one group by number or by name — `null`
for a declared group that did not take part — `groups()` every group in `preg_match`'s order, and
`offset()` where the match starts, counted in graphemes.

```nvs
<?nvs
var $m = Core\Regex::match("café au lait", "(?<drink>lait)");
if ($m != null) {
    echo $m->text(), " ", $m->offset(), " ", $m->group("drink") ?? "?", " ", $m->group(1) ?? "?", "\n";
    foreach ($m->groups() as string $k => ?string $v) {
        echo $k, "=", $v ?? "null", ";";
    }
    echo "\n";
}
var $opt = Core\Regex::match("ac", "(a)(b)?(c)");
echo $opt?->group(2) ?? "no b", " ", $opt?->group(3) ?? "no c", "\n";
```
```output
lait 8 lait lait
0=lait;drink=lait;1=lait;
no b c
```
