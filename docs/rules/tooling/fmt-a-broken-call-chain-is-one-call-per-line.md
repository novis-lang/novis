A chain of two or more `->` or `?->` calls has two layouts: on one line, or with every arrow starting a
line of its own. It is broken when a line break its author wrote sits before any one of its arrows.

A broken chain keeps its receiver on the line the chain starts on, and starts each arrow on a line one
level in from that line. The arguments of each call are a list under
`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`, judged on their own: a broken chain does
not break them, and a broken argument list does not break the chain.

```nvs
$rows = $query->from('orders')->where('state', 'open')
    ->orderBy('created')->limit(20)->fetch();

$rows = $query
    ->from('orders')
    ->where('state', 'open')
    ->orderBy('created')
    ->limit(20)
    ->fetch();
```

The first statement formats to the second. A chain written on one line stays on one line.
