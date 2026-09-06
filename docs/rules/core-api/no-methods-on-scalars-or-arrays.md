A scalar and an `array<T>` never gain methods. There is no `$s->length()` and no `$a->map()`; `Core\Str`
and `Core\Arr` are the one spelling for both.

The alternative is a second surface that grows in parallel with the first forever, which is how the twin
problem regrows after it has been removed once. Keeping the operations on the domain class also keeps them
where the subject-first rule (`rule:core-api/subject-first`) puts them, so a reader learns one call shape
rather than two. Member access on a value that has no members is refused where it is written
(`rule:types/erased-member-access`).
