`exec($cmd)`, `system($cmd)`, `` `cmd` `` and `shell_exec($cmd)` map to `Core\Process::run($path,
$argv)` (`rule:core-classes/process-run`). `passthru($cmd)` maps to `Core\Process::spawn($path,
$argv)` streaming `readStdout()` to the response, and `proc_open($cmd, $descriptors, $pipes)` to
`spawn` plus the handle's read, write, wait and kill (`rule:core-classes/process-spawn`); PHP's
descriptor-spec array has no one-to-one structural match.

Every one of these sites is flagged for human review rather than rewritten. A shell command string
has no mechanical argv split — quoting, globbing, `&&` and `|` are shell semantics with no argv
equivalent — and its meaning depends on a shell grammar `nvs convert` does not interpret and Novis
never will (`rule:core-classes/process-is-argv-only`). A human supplies the real executable path and
the argv; the converter supplies the call shape and the pointer.
