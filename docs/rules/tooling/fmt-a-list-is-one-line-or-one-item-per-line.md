A list has two layouts: every item on one line, or one item per line. The lists are a call's
arguments, including `new`'s; the parameter list of a function, a method or an anonymous function; an
array literal; an anonymous object; and a shape type.

A list is **broken** when a line break its author wrote sits at the list's own level: after the
opener, between two items, or before the closer. A break inside an item — a function body, a nested
array, a nested call, a heredoc — belongs to that item and never breaks the list around it.

| Written | Formatted |
|---|---|
| no break at the list's own level | one line, however long |
| a break after the opener, between two items, or before the closer | the opener ends its line, each item starts a line one level in, a trailing comma follows the last item, and the closer starts a line at the indentation of the opener's line |

```nvs
$order = Shop::place($customer,
    $basket, $address);

$order = Shop::place(
    $customer,
    $basket,
    $address,
);
```

The first statement formats to the second. An item that spans several lines moves in with its first
line and keeps the layout its own rules give it, so a call whose last argument is a function with a
multi-line body and no break between its arguments stays as written:

```nvs
array<string> $names = Core\Arr::map($users, fn(User $user) {
    return Core\Str::upper($user->name);
});
```

A comment inside a broken list stays on the line of the item it follows, or keeps a line of its own
when it was written on one. A line comment breaks the list it sits in, because a line break follows
it. `match` arms and enum cases keep their own rule (`rule:tooling/fmt-novis-constructs`).
