Every chain of a binary operator has the layout `rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`
gives `&&`, `||` and `??`: on one line, or one operand per line with the operator at the start of the
line, one level in from the line the first operand starts on. The operators are the arithmetic ones,
the comparisons, `<=>`, the bitwise ones, `|>`, `&&`, `||`, `??` and `.`. A chain is the run at the
lowest precedence, and it is broken by a line break its author wrote between two of its operands, on
either side of an operator.

```nvs
$total = $total +
    ((Core\Bytes::contains($message, $needles[($total % 3) as int]) ? 2 : 1) as uint);

$total = $total
    + ((Core\Bytes::contains($message, $needles[($total % 3) as int]) ? 2 : 1) as uint);
```

The first statement formats to the second. A comparison has two operands, so it is a chain of two:
`$order->shippedAt() - $order->paidAt() >\n$policy->maxDelay()` puts `> $policy->maxDelay()` on a line
of its own, and the subtraction, which is not broken, stays on one line.

**A broken chain inside a broken chain is one level further in.** An operand made of higher-precedence
operators is a chain of its own. When it is broken too, its operators start their lines one level in
from its first operand, so the reader sees which operator binds tighter. The formatter never adds
parentheses, because that would change the tokens (`rule:tooling/fmt-one-run-between-two-tokens`).

```nvs
$free = $isMember
    || $hasCoupon
        && $price * $quantity > 50;
```

**`.` keeps its author's lines.** A broken `.` chain is not one operand per line. Every line break its
author wrote between two operands stays, the `.` moves to the start of the line after it, and operands
written on one line stay on that line. A message built from many short pieces stays as wide as its
author wrote it:

```nvs
$message = 'Order ' . $order->id() . ' for ' . $customer->name() .
    ' was paid on ' . $order->paidAt() . ' with ' . $order->method();

$message = 'Order ' . $order->id() . ' for ' . $customer->name()
    . ' was paid on ' . $order->paidAt() . ' with ' . $order->method();
```

So a broken `.` chain is the one construct with three layouts: one line, one operand per line, or the
lines its author chose. The cost is accepted because a string is built from pieces of very different
lengths, and one piece per line turns a sentence into a column of four-character lines.

`is` and `as` are not chains. Their line breaks are the author's, as
`rule:tooling/fmt-every-construct-has-a-broken-layout` lists.

**Declined:**
- **Leaving arithmetic and comparisons as written.** A trailing `+` and a leading `+` stay two
  layouts of one program.
- **Adding parentheses around a broken higher-precedence group.** It changes the tokens, and the
  indentation shows the same grouping.
- **One operand per line for `.` too.** It splits `'Order ' . $order->id()` into two lines that read as
  one phrase.
