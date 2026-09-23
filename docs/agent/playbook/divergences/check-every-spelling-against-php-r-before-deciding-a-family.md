- **Check every spelling against `php -r` before deciding a family is refused, because PHP does not
  always agree with itself.** `echo $a[];` and `unset($a[])` are compile errors, but `$a[] .= "x"`
  appends silently even at `error_reporting=-1`, so refusing the third is a divergence that needs
  its own row under `rule:php-migration/every-divergence-is-deliberate-and-listed`, not a
  doc-comment sentence. A probe under `.agent-tmp/` run through both `nvs run` and `php` settles a
  paragraph before it is written. [until: reviewed 2026-09-06]
