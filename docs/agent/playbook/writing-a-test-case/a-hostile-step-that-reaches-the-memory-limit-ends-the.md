- **A hostile step that reaches the memory limit ends the program, so every step written below it
  never runs and nothing says so.** An `array<Level>` filled with ten million cases stops at
  `FATAL: the request exceeded its memory limit`, and the file then passes as soon as
  `// hostile: ends-early` is declared, while proving one step of the four it claims in its own top
  comment. Put the step that cannot be caught last, and read the program's own output before
  declaring `ends-early`. [until: gone tools/nv/proofs/run.ts:endsEarly]
