Names the form a language uses for a given count, so one message can be written correctly in every
language.

English has two forms: "1 file" and "3 files". Russian has four, and picks between them by rules that
put 21 with 1 and 11 with 5. Welsh uses six. `Core\Cldr::pluralCategory` answers which form a count
selects in a language, as one of `Zero`, `One`, `Two`, `Few`, `Many` and `Other`, and your message
catalog keeps one line per form.

**In plain words:** these names are labels for a language's forms, not amounts. `Many` does not mean
"a lot", and `One` is whichever counts that language treats the way English treats 1 — in Russian
that includes 21 and 101.

**Good to know:** every language has `Other`, and a language that makes no plural distinction at all
uses only that one.
