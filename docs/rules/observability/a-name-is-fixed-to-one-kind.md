A metric name is fixed to one kind — counter, histogram or gauge — on its first use within a
process, and its label *names* are fixed with it. A second kind for the same name is a runtime throw
naming both call sites, because it is a mistake and not a mode: the exposition formats both
exporters write have no way to say that two members of one family disagree about what they are, so
a series recorded two ways is a bug the backend reports and the call site cannot see.

A label-set mismatch is refused on the same footing. Label *order* is not a second series — the
registry keys on the set, not the spelling. Both refusals are distinct from the cardinality
refusal of `rule:observability/past-max-series-a-new-series-is-refused`, which is a no-op rather
than a throw.
