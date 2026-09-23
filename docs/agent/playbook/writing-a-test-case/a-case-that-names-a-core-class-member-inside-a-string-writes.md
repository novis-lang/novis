- **A case that names a `Core\Class::member` inside a *string* writes the literal single-quoted.**
  Single-quoted, only `\\` and `\'` are escapes (PHP's rule), so `'Core\Time::fromIso(): '` is
  exactly those characters. `gaps.py --errors` drops a site only when the literal run before the
  message's first format hole appears in the suite, so assert `Core\Str::startsWith($e->message,
  '…')` with the prefix spelled out, not a tail a dependency bump will rewrite.
  [until: reviewed 2026-09-06]
