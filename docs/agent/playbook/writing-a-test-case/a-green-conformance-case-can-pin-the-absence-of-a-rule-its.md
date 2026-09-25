- **A green conformance case can pin the absence of a rule its own record requires, and reads as
  coverage, not a gap.** `an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst` asserted what
  the compiler did before `rule:attributes/structural-retrieval`'s last paragraph existed in code,
  so landing it turned the case red. Before landing a rule the acceptance list names, `grep -rn` the
  `.nvst` corpus for the spelling it will start refusing. [until: gone tests/conformance/core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst:rule:attributes/structural-retrieval]
