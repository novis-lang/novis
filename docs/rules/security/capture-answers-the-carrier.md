Capturing output answers **the carrier of the sink in force**, not a plain `string`. The reason is not
symmetry: the captured bytes have *already* been through the sink, so handing them back as a `string`
and re-emitting them would escape them a second time and corrupt the page.

The same rule gives a spawned isolate's captured result its type
(`rule:security/isolate-output-is-captured`), by the same argument and with the same fix. Inheriting
rather than capturing needs no rule at all: the child's carrier is the parent's, so appending composes
the way carrier addition already does.

This is the third instance of one rule rather than three members that each argued it alone — the other
two being a laundering escape and an approved URL — and stating it once is what keeps a future
capturing member from answering the wrong type (`rule:security/launderer-answers-a-carrier`).
