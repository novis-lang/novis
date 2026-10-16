The error a program raises when the mistake is its own.

A base that does not exist, a depth outside the range a member allows, a unit too small to move the
value it was handed: these are faults in the code, not in the world around it. Novis gives them their
own half of the exception tree, so a program can tell *we got this wrong* from *something outside
said no* without reading the message and guessing.

Catching one is usually the wrong instinct: the honest answer to a bug is to fix the call. Work that
has to finish anyway is the exception, and it records the fault as its own.

**Good to know:** it sits *beside* `RuntimeError` under `Throwable` rather than under it, so a clause
written for runtime errors never takes one, and neither half contains the other.

**The examples below** show a call the library refuses, the two clauses that tell our fault from the
world's, and a report that finishes and names the rows whose settings were wrong.
