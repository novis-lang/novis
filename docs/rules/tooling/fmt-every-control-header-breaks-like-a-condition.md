The parenthesised header of every control structure — `if`, `elseif`, `while`, `do … while`,
`switch`, `match`, `foreach` and `for` — has two layouts. It is on one line, or its `(` ends its line,
its parts start lines one level in, and its `)` starts a line of its own, followed by ` {`.
`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line` gives this layout to a condition, and
this rule gives it to every header.

A header is broken by a line break its author wrote after the `(`, before the `)`, or inside an
operator chain anywhere in the header that is not inside brackets of its own. A break inside a call's
arguments, an array or an anonymous function's body belongs to that item, and leaves the header on
its line. So a break inside a higher-precedence group breaks the condition too:

```nvs
if ($isMember || $hasCoupon &&
    $price * $quantity > 50) {

if (
    $isMember
    || $hasCoupon
        && $price * $quantity > 50
) {
```

The parts of each header:

| Header | Its parts, one per line |
|---|---|
| `if`, `elseif`, `while`, `do … while`, `switch`, `match` | the expression, laid out as its own chain |
| `foreach` | the expression, then `as` and the binding, with `as` starting its line |
| `for` | each clause, ending in its `;`, and the last one without |

```nvs
foreach (
    Core\Arr::filter($orders, fn(Order $order): bool => $order->isPaid())
    as Order $order
) {

$label = match (
    $order->state()
    ?? $order->previousState()
    ?? State::Unknown
) {

for (
    $i = 0;
    $i < Core\Arr::count($orders);
    $i++
) {
```

A `foreach` with a key keeps `$key => $value` together on the `as` line. The `for` layout is PER-CS's.

**Declined:**
- **The parenthesis layout for `if`, `elseif`, `while` and `do … while` alone.** A broken `match`
  subject then leaves its `)` on its last operand's line, which is the layout this rule replaces.
- **A break inside a higher-precedence group leaving the condition unbroken.** The group's operator
  starts a line under the `if`, where it reads as part of the top-level chain.
