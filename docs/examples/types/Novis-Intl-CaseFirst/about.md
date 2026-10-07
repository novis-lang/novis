Whether uppercase or lowercase letters come first when `Collator` sorts two words that differ only
in case.

You pass a `CaseFirst` to `Collator::sort` or `Collator::sortKeys` as `caseFirst`. `Off`, the
default, uses the order of the locale. In English that puts "apple" before "Apple". `Upper` puts
"Apple" first, and `Lower` puts "apple" first.

The setting changes only the order of words that are equal apart from case. "Apple" still comes
before "berry" with every case.

**Good to know:** with `strength: Strength::Primary`, case does not count at all, so this setting
has no effect.
