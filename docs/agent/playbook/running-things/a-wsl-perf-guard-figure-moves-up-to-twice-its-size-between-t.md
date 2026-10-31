- **A WSL perf-guard figure moves up to twice its size between two runs of the same tree.** The
  platform heap read 5.8 ns, 11.2 ns and 30.5 ns on three steady runs, and the balance update 1.3 ns
  and 2.3 ns on one unchanged tree, so one before run and one after run cannot show a change in a few
  nanoseconds. Compare the function's disassembly before and after with `objdump -d` on the binary
  under `/var/tmp/nvs-target-wsl/release/deps/`, and stash the change while the before tree syncs
  (`tools/nv/cmd/linux-test.ts`).
