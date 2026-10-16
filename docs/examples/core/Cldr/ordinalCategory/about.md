Tells you which form a number takes when it is a place rather than an amount, such as English's
`1st`, `2nd`, `3rd` and `4th`.

`ordinalCategory` returns the form for the place you give it: `One`, `Two`, `Few`, `Many`, `Zero`
or `Other`. English uses four of them, so a template picks `st`, `nd`, `rd` or `th` from the answer.
Most languages mark no place at all and return `Other` for every number. German writes `3.` for
every place, and Japanese writes the number alone.

The second argument is a language tag such as `"en"`, `"it"` or `"cy"`. It is read the same way
`Core\Cldr::pluralCategory` reads it: only the language part, and upper and lower case are the same.
A language Novis carries no rules for throws a `LogicError`.

**Good to know:** the form for a place is not the form for an amount. In English, 3 books is
`Other` and 3rd place is `Few`. Use this member for places and `Core\Cldr::pluralCategory` for
amounts.

**The examples below** show the English suffixes, then how the two members differ for the same
number, then a results list written in two languages.
