- **A hostile step that hands a member an outsized key *and* an outsized subject multiplies the
  two.** `Core\Arr::column` hashes the column key once per row, so a key of a million characters
  over a 200000-row table is 200 GB of hashing and had not finished after two minutes, while each
  half on its own takes a second. Give an outsized input its own three-entry subject, and keep the
  big subject's keys ordinary. [until: reviewed 2026-09-20]
