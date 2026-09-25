`benches/userland/` holds ordinary web-and-CLI programs written once per engine — Novis, PHP and
Python — and `bun nv bench` runs whichever engines a case has twins for and prints the
same-host ratios. That is where a comparison against another engine is made, and it is the only
place one is made.

It changes nothing about the historical dashboard, whose headline metric stays the instruction
count. Which workloads populate that dashboard is a benchmark-design question rather than a language
decision, and it grows as real compiled programs exist to measure.
