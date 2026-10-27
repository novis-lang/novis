//! A `->` call chain is on one line or has one call per line
//! (`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`), and the
//! author's line break before an arrow is what breaks it. A chain of any binary
//! operator is on one line or has one operand per line, operator first, and a
//! `.` chain keeps its author's lines
//! (`rule:tooling/fmt-every-binary-operator-breaks-operator-first`). A
//! ternary is on one line or has a line for its `?` and one for its `:`
//! (`rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch`).
//!
//! Each case is the smallest file that shows one half of the rule: what counts
//! as a break in a chain, what does not, and the one layout a broken chain
//! gets.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// Asserts `written` formats to `canonical`, and `canonical` to itself.
fn formats_to(written: &str, canonical: &str) {
    assert_eq!(formatted(written), canonical);
    assert_eq!(
        formatted(canonical),
        canonical,
        "formatting is a fixed point"
    );
}

#[test]
fn a_call_chain_on_one_line_stays_on_one_line() {
    let wide = "<?nvs\n$rows = $query->from('orders')->where('state', 'open')->orderBy('created')->limit(20)->fetch();\n";
    formats_to(wide, wide);
    let nullsafe = "<?nvs\n$name = $user?->profile()?->name();\n";
    formats_to(nullsafe, nullsafe);
}

#[test]
fn a_break_before_one_arrow_puts_every_call_on_its_own_line() {
    formats_to(
        "<?nvs\n$rows = $query->from('orders')->where('state', 'open')\n    ->orderBy('created')->limit(20)->fetch();\n",
        "<?nvs\n$rows = $query\n    ->from('orders')\n    ->where('state', 'open')\n    ->orderBy('created')\n    ->limit(20)\n    ->fetch();\n",
    );
    formats_to(
        "<?nvs\nclass Report\n{\n    public function first(Shop $shop): ?string\n    {\n        return $shop->orders?->first()\n?->name();\n    }\n}\n",
        "<?nvs\nclass Report\n{\n    public function first(Shop $shop): ?string\n    {\n        return $shop->orders\n            ?->first()\n            ?->name();\n    }\n}\n",
    );
}

#[test]
fn a_broken_argument_list_does_not_break_the_chain() {
    formats_to(
        "<?nvs\n$rows = $query->where('state',\n'open')->fetch();\n",
        "<?nvs\n$rows = $query->where(\n    'state',\n    'open',\n)->fetch();\n",
    );
    formats_to(
        "<?nvs\n$rows = $query\n->where('state', 'open')->fetch();\n",
        "<?nvs\n$rows = $query\n    ->where('state', 'open')\n    ->fetch();\n",
    );
}

#[test]
fn an_operator_chain_on_one_line_stays_on_one_line() {
    let wide = "<?nvs\nvar $ready = $user->isActive() && $user->hasRole('admin') && $request->isSecure() && $request->isFresh();\n";
    formats_to(wide, wide);
    let label = "<?nvs\nvar $label = $first . ' ' . $last ?? 'nobody';\n";
    formats_to(label, label);
}

#[test]
fn a_break_between_two_operands_puts_every_operand_on_its_own_line_operator_first() {
    formats_to(
        "<?nvs\nvar $ready = $user->isActive() &&\n    $user->hasRole('admin') && $request->isSecure();\n",
        "<?nvs\nvar $ready = $user->isActive()\n    && $user->hasRole('admin')\n    && $request->isSecure();\n",
    );
    formats_to(
        "<?nvs\necho 'Hello, ' . $name\n. '!', \"\\n\";\n",
        "<?nvs\necho 'Hello, ' . $name\n    . '!', \"\\n\";\n",
    );
    formats_to(
        "<?nvs\nvar $name = $given ?? $nick\n?? 'nobody';\n",
        "<?nvs\nvar $name = $given\n    ?? $nick\n    ?? 'nobody';\n",
    );
}

#[test]
fn a_mixed_operator_run_breaks_at_its_lowest_precedence() {
    formats_to(
        "<?nvs\nvar $allowed = $user->isAdmin() || $user->isOwner() && $post->isOpen() ||\n    $post->isPublic();\n",
        "<?nvs\nvar $allowed = $user->isAdmin()\n    || $user->isOwner() && $post->isOpen()\n    || $post->isPublic();\n",
    );
    formats_to(
        "<?nvs\nvar $allowed = $user->isAdmin() || $user->isOwner() &&\n    $post->isOpen();\n",
        "<?nvs\nvar $allowed = $user->isAdmin() || $user->isOwner()\n    && $post->isOpen();\n",
    );
}

#[test]
fn a_broken_arithmetic_chain_is_one_operand_per_line_operator_first() {
    formats_to(
        "<?nvs\n$total = $total +\n((Core\\Bytes::contains($message, $needle) ? 2 : 1) as uint);\n",
        "<?nvs\n$total = $total\n    + ((Core\\Bytes::contains($message, $needle) ? 2 : 1) as uint);\n",
    );
    formats_to(
        "<?nvs\n$total = $price + $shipping\n- $discount + $tax;\n",
        "<?nvs\n$total = $price\n    + $shipping\n    - $discount\n    + $tax;\n",
    );
}

#[test]
fn a_broken_comparison_is_a_chain_of_two() {
    formats_to(
        "<?nvs\n$late = $order->shippedAt() - $order->paidAt() >\n$policy->maxDelay();\n",
        "<?nvs\n$late = $order->shippedAt() - $order->paidAt()\n    > $policy->maxDelay();\n",
    );
}

#[test]
fn a_broken_chain_inside_a_broken_chain_is_one_level_further_in() {
    let canonical = "<?nvs\n$free = $isMember\n    || $hasCoupon\n        && $price * $quantity > 50;\n";
    formats_to(
        "<?nvs\n$free = $isMember ||\n$hasCoupon &&\n$price * $quantity > 50;\n",
        canonical,
    );
    formats_to(
        "<?nvs\n$free = $isMember ||\n$hasCoupon\n&& $price * $quantity > 50;\n",
        canonical,
    );
    formats_to(
        "<?nvs\nif ($isMember\n|| $hasCoupon\n&& $price > 50) {\n    echo 1;\n}\n",
        "<?nvs\nif (\n    $isMember\n    || $hasCoupon\n        && $price > 50\n) {\n    echo 1;\n}\n",
    );
}

#[test]
fn a_broken_dot_chain_keeps_its_authors_lines_with_the_dot_first() {
    let canonical = "<?nvs\n$message = 'Order ' . $order->id() . ' for ' . $customer->name()\n    . ' was paid on ' . $order->paidAt();\n";
    formats_to(
        "<?nvs\n$message = 'Order ' . $order->id() . ' for ' . $customer->name() .\n    ' was paid on ' . $order->paidAt();\n",
        canonical,
    );
    formats_to(
        "<?nvs\n$message = 'Order ' . $order->id() . ' for ' . $customer->name()\n. ' was paid on ' . $order->paidAt();\n",
        canonical,
    );
    formats_to(
        "<?nvs\n$message = 'Order ' . $order->id() .\n' for ' . $customer->name()\n        . ' was paid on ' . $order->paidAt();\n",
        "<?nvs\n$message = 'Order ' . $order->id()\n    . ' for ' . $customer->name()\n    . ' was paid on ' . $order->paidAt();\n",
    );
}

#[test]
fn a_broken_condition_puts_its_parentheses_on_lines_of_their_own() {
    formats_to(
        "\
<?nvs
if ($user->isActive() &&
    $user->hasRole('admin') && $request->isSecure()) {
    Audit::log($user);
}
",
        "\
<?nvs
if (
    $user->isActive()
    && $user->hasRole('admin')
    && $request->isSecure()
) {
    Audit::log($user);
}
",
    );
    formats_to(
        "<?nvs\nwhile ($queue->hasNext()\n|| $retry) {\n    $queue->next();\n}\n",
        "<?nvs\nwhile (\n    $queue->hasNext()\n    || $retry\n) {\n    $queue->next();\n}\n",
    );
    formats_to(
        "<?nvs\nif ($a) {\n    echo 1;\n} elseif ($b\n&& $c) {\n    echo 2;\n}\n",
        "<?nvs\nif ($a) {\n    echo 1;\n} elseif (\n    $b\n    && $c\n) {\n    echo 2;\n}\n",
    );
    formats_to(
        "<?nvs\ndo {\n    $queue->next();\n} while ($queue->hasNext() &&\n$retry);\n",
        "<?nvs\ndo {\n    $queue->next();\n} while (\n    $queue->hasNext()\n    && $retry\n);\n",
    );
}

#[test]
fn a_break_after_the_condition_parenthesis_breaks_the_condition() {
    formats_to(
        "<?nvs\nif (\n$user->isActive() && $request->isSecure()) {\n    Audit::log($user);\n}\n",
        "<?nvs\nif (\n    $user->isActive()\n    && $request->isSecure()\n) {\n    Audit::log($user);\n}\n",
    );
    formats_to(
        "<?nvs\nif ($user->isActive()\n) {\n    Audit::log($user);\n}\n",
        "<?nvs\nif (\n    $user->isActive()\n) {\n    Audit::log($user);\n}\n",
    );
}

#[test]
fn a_break_inside_one_operand_does_not_break_the_condition() {
    formats_to(
        "<?nvs\nif ($user->hasRole('admin',\n'owner') && $request->isSecure()) {\n    Audit::log($user);\n}\n",
        "<?nvs\nif ($user->hasRole(\n    'admin',\n    'owner',\n) && $request->isSecure()) {\n    Audit::log($user);\n}\n",
    );
    let one_line =
        "<?nvs\nif ($user->isActive() && $request->isSecure()) {\n    Audit::log($user);\n}\n";
    formats_to(one_line, one_line);
}

#[test]
fn a_broken_chain_is_a_fixed_point() {
    let condition = "\
<?nvs
if (
    $user->isActive()

    // only over a secure connection
    && $request->isSecure()
) {
    Audit::log($user);
}
";
    formats_to(
        "<?nvs\nif ($user->isActive()\n\n// only over a secure connection\n&& $request->isSecure()) {\n    Audit::log($user);\n}\n",
        condition,
    );
    let canonical = "\
<?nvs
$rows = $query
    ->from('orders')

    // only the open ones
    ->where('state', 'open')
    ->fetch();
";
    formats_to(
        "<?nvs\n$rows = $query->from('orders')\n\n// only the open ones\n->where('state', 'open')->fetch();\n",
        canonical,
    );
}

#[test]
fn a_ternary_on_one_line_stays_on_one_line() {
    let wide = "<?nvs\necho $room * 2 > $limit ? 'more than half is free' : 'less than half is free';\n";
    formats_to(wide, wide);
    let short = "<?nvs\n$name = $given ?: 'nobody';\n";
    formats_to(short, short);
}

#[test]
fn a_ternary_broken_after_its_condition_has_a_line_per_branch() {
    formats_to(
        "<?nvs\necho $room * 2 > $limit\n? 'more' : 'less';\n",
        "<?nvs\necho $room * 2 > $limit\n    ? 'more'\n    : 'less';\n",
    );
    formats_to(
        "<?nvs\necho $room * 2 > $limit ?\n'more' : 'less';\n",
        "<?nvs\necho $room * 2 > $limit\n    ? 'more'\n    : 'less';\n",
    );
}

#[test]
fn a_ternary_broken_before_its_colon_has_a_line_per_branch() {
    formats_to(
        "<?nvs\necho $room * 2 > $limit ? 'more'\n    : 'less';\n",
        "<?nvs\necho $room * 2 > $limit\n    ? 'more'\n    : 'less';\n",
    );
}

#[test]
fn a_broken_short_ternary_starts_a_line_with_its_operator() {
    formats_to(
        "<?nvs\n$name = $given ?:\n'nobody';\n",
        "<?nvs\n$name = $given\n    ?: 'nobody';\n",
    );
}

#[test]
fn a_broken_chain_in_a_ternary_branch_is_one_level_further_in() {
    formats_to(
        "<?nvs\n$label = $known\n? $first\n. ' ' . $last : 'nobody';\n",
        "<?nvs\n$label = $known\n    ? $first\n        . ' ' . $last\n    : 'nobody';\n",
    );
}
