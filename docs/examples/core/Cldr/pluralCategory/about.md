Tells you which form of a word a number needs in a language, so a translated message reads
correctly.

English has two forms: "1 message" and "2 messages". Other languages have more. Polish uses three
forms, Welsh uses six, and Japanese uses one. `pluralCategory` returns the form the language uses
for the count you give it: `One`, `Two`, `Few`, `Many`, `Zero` or `Other`. These are names for the
forms of a language, not amounts, so `Many` does not mean a large number.

The second argument is a language tag such as `"en"`, `"de"` or `"pt-BR"`. Only the language part
is read, and upper and lower case are the same. A language Novis carries no rules for throws a
`LogicError`, because a message shown with another language's forms is wrong.

**Good to know:** `1` and `1.0` can select different forms. In English `1` is `One` and `1.0` is
`Other`, because the rules read the digits a reader sees.

**The examples below** show one English message, then the three forms Polish uses, then a message
catalog that picks its line by form.
