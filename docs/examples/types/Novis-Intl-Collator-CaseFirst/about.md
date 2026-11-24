Whether uppercase or lowercase letters come first when `Collator` sorts two words that differ only
in case.

`Collator::CaseFirst` is declared in the class `Novis\Intl\Collator`.

`Collator` sorts strings by the rules of a language. You pass a `Collator::CaseFirst` to
`Collator::sort` or `Collator::sortKeys` as `caseFirst`. `Off` is the default. It follows the rules
of the language. In English, it puts "apple" before "Apple". `Upper` puts "Apple" first, and `Lower`
puts "apple" first.

The setting changes only the order of words that are equal apart from case. Other words are sorted
by their letters first. So "Apple" comes before "berry" with `Off`, `Upper` and `Lower`.

**Good to know:** with `strength: Collator::Strength::Primary`, case does not count at all, so this setting
has no effect.
