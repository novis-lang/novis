How many words `RelativeTime::format` and `ListFormat::join` use.

You pass a `Width` as `width`:

- `Wide` writes every word in full: "3 months ago" and "Shop, Blog, and Wiki". This is the
  default.
- `Short` uses short forms: "3 mo. ago" and "Shop, Blog, & Wiki".
- `Narrow` uses the shortest forms: "3mo ago" and "Shop, Blog, Wiki".

Use `Short` or `Narrow` where space is small, such as on a phone screen or in a table column.

**Good to know:** in some locales, two widths give the same text.
