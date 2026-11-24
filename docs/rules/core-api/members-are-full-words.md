A name is a full word, so `Core\Str::length`, never `Core\Str::len`. The rule holds for every member
name, every option field name and every class name of a first-party package (`extensions/*/nvs/`,
`packages/rest/`), as well as `Core`. An enum case is held to it too: `Weekday::Monday`, `Month::January`,
`Unit::Millisecond`. A `Core` class name may abbreviate, but only from a closed list: `Str`, `Arr`, `Fs`,
`Io`, `Uri`, `Db`, `Id`.

Three closed lists in `tools/data/api-names.json` settle every short word, and `bun nv names --check`
(`tools/nv/cmd/names.ts`) reads them from there:

- **`mathNames`**, the conventional mathematical names (`abs`, `min`, `max`, `sqrt`, `gcd`, `atan2` and
  the rest), are allowed as a whole member name and nowhere else, so `Math::max` but never `maxLength`.
  `mathWordsAlsoEnglish` lists the ones that are also English words (`log`, `floor`), which may stand
  anywhere.
- **`acronyms`** are a standard's own name and count as one word: `json`, `uri`, `uuid`, `sql`, `hmac`,
  `dn`, `v4` and the rest.
- **`truncations`** are banned, and the check's message names the full word for each: `dir` is
  `directory`, `args` is `arguments`, `nanos` is `nanoseconds`, `cwd` is `workingDirectory`, `diff` is
  `difference`, `pow` is `power`.

A word on neither list is judged by the person who adds it, against a dictionary. `info`, `stats` and
`app` are dictionary words, so `Reflect\ClassInfo` and `Queue\Stats` keep their names. A word that is both
a truncation and an allowed short word is an error in the tables, and the command stops on it.

Abbreviation is where a naming convention stops being mechanical: decided one member at a time, it
produces `len` beside `wordCount` in the same class. Closed lists remove the judgement call, at the cost
of a few extra characters at every call site.
