/**
 * Hand-maintained corrections for the Core reference that `bun nv render
 * --website` reads out of the spec (tools/nv/renderers/website-core.ts).
 *
 * The spec (docs/spec/01-core-library.md) states some signatures only in
 * prose, pairs two members in one table row, or writes a roster the renderer
 * cannot attribute mechanically. Every such case is corrected HERE, never in
 * the renderer — so a spec edit that invalidates one of these shows up as a
 * render warning and this file is the one place to fix it.
 *
 * Signatures written here are taken from the spec's own prose or from the
 * implementing module's doc comments in crates/nvs-stdlib/src/ — each entry
 * says which.
 */

export const overrides = {
  /** `$receiver->` name → owning class, for tables outside the class's own heading. */
  receivers: {
    match: 'Core\\Regex\\Match',
    stream: 'Core\\Hash\\Stream',
    uri: 'Core\\Uri',
    uuid: 'Core\\Uuid',
    z: 'Core\\Time\\Zone',
  },

  /**
   * Per-row corrections, keyed by the member cell's text without backticks.
   *   skip:      drop the row entirely (its members come from `members` below)
   *   silent:    keep the parse result but suppress the row's warnings
   *   class:     force the owning class
   *   cloneAlso: extra names in a `a` / `b` cell that share a's exact signature
   */
  rows: {
    // ---- § 3 Core\Math: "hyperbolic, same shape" is not a signature.
    'sinh cosh tanh': { skip: true },

    // ---- § 4 Core\Time\Instant / DateTime: `a` / `b` cells that do share one shape.
    'plus / minus': { cloneAlso: ['minus'] },
    'next / previous': { cloneAlso: ['previous'] },
    'startOf / endOf': { cloneAlso: ['endOf'] },
    // `$d->date(): Date` does not clone to timeOfDay/zone (different returns).
    'date / timeOfDay / zone': { silent: true },
    // Duration/Zone shared table: receivers disambiguated per row.
    '$d->toSeconds': { class: 'Core\\Time\\Duration' },
    '$d->plus / minus / multipliedBy / negated': { class: 'Core\\Time\\Duration' },
    'Zone::UTC': { skip: true }, // a constant, not a member — see classes below

    // ---- § 7: the section heading names two classes, so every row states its
    // class; a combined row states both halves' signatures in its own cell.
    decodeText: { class: 'Core\\Encoding' },
    encodeText: { class: 'Core\\Encoding' },
    isValidText: { class: 'Core\\Encoding' },
    'toBase64 / fromBase64': { class: 'Core\\Encoding' },
    'toBase64Url / fromBase64Url': { class: 'Core\\Encoding' },
    'toHex / fromHex': { class: 'Core\\Encoding' },
    'toBase32 / fromBase32': { class: 'Core\\Encoding' },

    // ---- § 12 Core\Uri: reader rows hold several signatures in one cell.
    '$uri->scheme / $uri->userInfo / $uri->host / $uri->port': { skip: true },
    '$uri->path / $uri->query / $uri->fragment / $uri->toString': { skip: true },
    'Uri::encodeComponent / decodeComponent': { silent: true },
    'Uri::encodeFormValue / decodeFormValue': { silent: true },
  },

  /**
   * Members the parser cannot read out of a table row. `signature` uses the
   * spec's own signature grammar and must parse.
   */
  members: [
    // ---- § 3 Core\Math hyperbolics — signatures from crates/nvs-stdlib/src/math.rs doc comments.
    { class: 'Core\\Math', signature: 'sinh(float $n): float', replaces: '`sinh`', qualifier: 'neutral' },
    { class: 'Core\\Math', signature: 'cosh(float $n): float', replaces: '`cosh`', qualifier: 'neutral' },
    { class: 'Core\\Math', signature: 'tanh(float $n): float', replaces: '`tanh`', qualifier: 'neutral' },
    { class: 'Core\\Math', signature: 'asinh(float $n): float', replaces: '`asinh`', qualifier: 'neutral' },
    { class: 'Core\\Math', signature: 'acosh(float $n): float', replaces: '`acosh`', qualifier: 'neutral' },
    { class: 'Core\\Math', signature: 'atanh(float $n): float', replaces: '`atanh`', qualifier: 'neutral' },

    // ---- § 4 Core\Time\Instant — "plus toEpochMicros" from the toEpochMillis row's note.
    { class: 'Core\\Time\\Instant', signature: '$i->toEpochMicros(): int', replaces: "`microtime(true)`'s two halves" },

    // ---- § 4 Core\Time\DateTime — the two readers the `date / timeOfDay / zone` row implies.
    { class: 'Core\\Time\\DateTime', signature: '$d->timeOfDay(): TimeOfDay', notes: 'component view' },
    { class: 'Core\\Time\\DateTime', signature: '$d->zone(): Zone', notes: 'component view' },

    // ---- § 4 Core\Time\Duration — from the `Duration::seconds` row's own note.
    { class: 'Core\\Time\\Duration', signature: 'nanoseconds(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'microseconds(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'milliseconds(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'minutes(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'hours(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'days(int $n): Duration' },
    { class: 'Core\\Time\\Duration', signature: 'weeks(int $n): Duration' },
    // From the `$d->toSeconds` row's note.
    { class: 'Core\\Time\\Duration', signature: '$d->toMilliseconds(): int' },
    { class: 'Core\\Time\\Duration', signature: '$d->toMicroseconds(): int' },
    { class: 'Core\\Time\\Duration', signature: '$d->toNanoseconds(): int' },

    // ---- § 4 Core\Time\Date / TimeOfDay constructors, stated in the section prose.
    { class: 'Core\\Time\\Date', signature: 'at(int $y, uint $m, uint $d): Date', notes: 'constructing an invalid date throws' },
    { class: 'Core\\Time\\TimeOfDay', signature: 'at(uint $hour, uint $minute, {second?: uint, nanos?: uint}): TimeOfDay' },


    // ---- § 7 Core\Bytes — the roster is prose in the spec; signatures from crates/nvs-stdlib/src/bytes.rs.
    { class: 'Core\\Bytes', signature: 'length(bytes $b): uint', replaces: '`strlen` over binary strings', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'at(bytes $b, int $index): uint', notes: 'answers the octet as a `uint`, not a one-byte buffer' },
    { class: 'Core\\Bytes', signature: 'slice(bytes $b, int $offset, ?int $length = null): bytes', replaces: '`substr` over binary strings' },
    { class: 'Core\\Bytes', signature: 'indexOf(bytes $haystack, bytes $needle, {from?: int}): ?uint', notes: 'no `caseInsensitive` option — case is a text concept', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'compare(bytes $a, bytes $b): int', notes: 'lexicographic octet order', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'contains(bytes $haystack, bytes $needle): bool', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'startsWith(bytes $b, bytes $prefix): bool', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'endsWith(bytes $b, bytes $suffix): bool', qualifier: 'neutral' },
    { class: 'Core\\Bytes', signature: 'join(array<bytes> $parts, bytes $separator = ""): bytes', replaces: '`implode` over binary strings' },
    { class: 'Core\\Bytes', signature: 'fill(uint $length, uint $byte): bytes', notes: 'a `$byte` above 255 throws' },
    { class: 'Core\\Bytes', signature: 'repeat(bytes $b, uint $times): bytes', replaces: '`str_repeat` over binary strings' },
    { class: 'Core\\Bytes', signature: 'pack(string $format, mixed ...$values): bytes', replaces: '`pack`', qualifier: 'sink (format)' },
    { class: 'Core\\Bytes', signature: 'unpack(bytes $b, string $format): array<mixed>', replaces: '`unpack`', qualifier: 'sink (format)' },

    // ---- § 12 Core\Uri — the reader rows, from that section's own prose.
    { class: 'Core\\Uri', signature: '$uri->scheme(): ?string', qualifier: 'neutral' },
    { class: 'Core\\Uri', signature: '$uri->userInfo(): ?string', qualifier: 'neutral', notes: 'one reader, not `user`/`pass` — RFC 3986 § 3.2.1 deprecates the split form' },
    { class: 'Core\\Uri', signature: '$uri->host(): ?string', qualifier: 'neutral', notes: '`null` where no authority was written, `""` where an empty one was' },
    { class: 'Core\\Uri', signature: '$uri->port(): ?int', qualifier: 'neutral' },
    { class: 'Core\\Uri', signature: '$uri->path(): string', qualifier: 'neutral' },
    { class: 'Core\\Uri', signature: '$uri->query(): ?string', qualifier: 'neutral' },
    { class: 'Core\\Uri', signature: '$uri->fragment(): ?string', qualifier: 'neutral' },
    { class: 'Core\\Uri', signature: '$uri->toString(): string', qualifier: 'neutral', replaces: 'reassembly by hand' },
    { class: 'Core\\Uri', signature: 'decodeComponent(string $s): string', replaces: '`rawurldecode`' },
    { class: 'Core\\Uri', signature: 'decodeFormValue(string $s): string', replaces: '`urldecode`' },

    // ---- § 12 Core\Validate — the roster is one prose line: all `(subject, …): bool`, all neutral.
    { class: 'Core\\Validate', signature: 'isEmail(string $s): bool', replaces: '`filter_var(…, FILTER_VALIDATE_EMAIL)`', qualifier: 'neutral' },
    { class: 'Core\\Validate', signature: 'isIp(string $s, {version?: 4\\|6}): bool', replaces: '`filter_var(…, FILTER_VALIDATE_IP)` and its flags', qualifier: 'neutral' },
    { class: 'Core\\Validate', signature: 'isMac(string $s): bool', replaces: '`filter_var(…, FILTER_VALIDATE_MAC)`', qualifier: 'neutral' },
    { class: 'Core\\Validate', signature: 'isDomain(string $s): bool', replaces: '`filter_var(…, FILTER_VALIDATE_DOMAIN)`', qualifier: 'neutral' },
    { class: 'Core\\Validate', signature: 'isAscii(string $s): bool', replaces: "`ctype_*`'s ASCII-range half", qualifier: 'neutral' },
    { class: 'Core\\Validate', signature: 'isPrintable(string $s): bool', replaces: "`ctype_print`", qualifier: 'neutral' },
  ],

  /** Enum name → owning class, where the spec's `Enums:` line sits in a
   * subsection whose class is not the enum's home. */
  enumOwners: {
    Weekday: 'Core\\Time',
    Month: 'Core\\Time',
    Unit: 'Core\\Time',
  },

  /** Class-level additions: constants and enums stated outside a parseable line. */
  classes: {
    'Core\\Time\\Zone': {
      constants: ['UTC'],
    },
    'Core\\Path': {
      constants: ['SEPARATOR'],
    },
  },
}
