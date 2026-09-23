- **An `--ORACLE-DIVERGES--` block is *one line*, however long the prose is.** The harness answers
  `not a valid case: line 3: `--ORACLE-DIVERGES--` is one line` and refuses the whole file, so a
  divergence written as paragraphs has to be folded into one. Write it as one line from the start
  with a capitalised lead-in (`THE INTEGER BAND:`) where a heading would have earned a break;
  `json-decode-refuses-the-number-band-json_decode-degrades.nvst` is the worked shape.
  [until: gone crates/nvs-test/src/case.rs:`--ORACLE-DIVERGES--` is one line]
