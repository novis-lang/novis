A member name is a full word. Class names may abbreviate, but only from a closed list — `Str`, `Arr`, `Fs`,
`Io`, `Uri`, `Db`, `Id` — and members may not, except the conventional mathematical spellings `abs`, `min`,
`max` and `sqrt`. So `Core\Str::length`, never `Core\Str::len`.

Abbreviation is where a naming convention stops being mechanical: PHP resolved it separately for every
function and produced `strlen` beside `str_word_count`. A closed list at the class level and no discretion
at the member level removes the judgement call entirely, at the cost of a few extra characters at every
call site.
