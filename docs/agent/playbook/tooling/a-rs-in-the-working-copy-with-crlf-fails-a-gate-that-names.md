- **A `.rs` in the working copy with CRLF fails a gate that names something else entirely.** Write
  on Windows, `Path.write_text`, or `nv splice` writing LF into a CRLF file all produce one; git
  normalises on commit, but `every_error_path_is_asserted_or_declared_unreachable` reads `Fault::`
  literals off disk, a `\`-continued string keeps the CR, and the failure names a message far from
  your edit with `\r\n` in the printed stem. One `python -c` byte rewrite of `\r\n` to `\n` fixes it
  (`newline=""` in a script prevents it); never `git stash` to bisect, because the loop driver may
  hold the tree. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:fault_sites]
