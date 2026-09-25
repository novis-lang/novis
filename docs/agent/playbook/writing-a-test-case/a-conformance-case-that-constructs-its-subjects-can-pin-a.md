- **A conformance case that *constructs* its subjects can pin a shape nobody wrote down.**
  `path-agrees-a-trailing-or-repeated-separator-is-nothing.nvst` doubles every separator of six
  paths, so widening `Core\Path`'s grammar with a UNC root turned one constructed path into a
  different root and four members stopped agreeing about it. After a grammar change read the cases
  that *build* their inputs before the ones that write them out; `nv verify`'s `.nvst` step is
  where it surfaces, a step after the unit tests that all passed. [until: gone tests/conformance/core/path-agrees-a-trailing-or-repeated-separator-is-nothing.nvst:Core\Path]
