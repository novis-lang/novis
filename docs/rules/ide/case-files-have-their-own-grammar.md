`.nvst` and `.lspt` get a second grammar: the section headers, with the Novis grammar embedded inside
the sections that hold a program — `--FILE--`, `--FILE <path>--`, `--SKIPIF--` and `--CLEAN--`. Every
other section is literal bytes with only its delimiter coloured. It is a thin wrapper whose bodies
`include` the grammar M4B builds anyway.

It ranks above the "nice later" pile because of who reads those files. This repository's own loop writes
hundreds of them and every session reads them as flat grey text, so the grammar that helps most per byte
written is the one for the format the project authors most — the only grammar here whose audience is the
people working on Novis rather than the people using it. A `.nvst` case opens with its sections coloured
and Novis highlighted inside `--FILE--`.
