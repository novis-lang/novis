- **`verify.py`'s test leg red on a *different* test each run, each of which "passed alone", is the
  box and not the tree.** Two runs here died on a lease-renewal test and then on a revalidation one,
  and the third was 13 of 13 green: the box was taking 133 ms to run `nvs --version` with the CPU at
  6%, so the disk was contended and no load average would have said so. Read `bench.py
  --warm-start`'s `start floor` before touching a timing assertion — near 5 ms is believable, tens of
  ms fails a different test every run. [until: reviewed 2026-09-19]
