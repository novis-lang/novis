The roster of features is read from live sources and kept nowhere. The registry's own machine-
readable answer names every registered class, member, exception, enum, interface and directive, so a
member that is not implemented is owed no proofs and one that lands is owed them at once. Every `#`
heading of the language and tool reference chapters names one language or tool feature, and those
chapters already state every rule the shipped compiler has, which makes their headings a roster that
maintains itself.

A source that stops naming a feature stops owing proofs for it; one that starts naming a new feature
owes them on the next sweep. Nobody edits a list, so no list is ever stale — a hand-kept roster is
wrong within a day of an unattended run, and it under-reports silently, which is the failure mode
that looks like success.
