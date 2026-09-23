- **A count the allocator is meant to refuse has to be past the address space, not merely past the
  RAM.** macOS backs a mapping lazily and says yes to a terabyte, so a case asking for one gets the
  allocation and then spends its whole 60-second budget writing the pages, where linux and windows
  refuse the same count outright and the case reads as green. Ask for a petabyte — still far under
  the `isize::MAX` seam, past every 64-bit address space — as
  `count-shaped-producers-refuse-alike.nvst` does.
  [until: gone tests/conformance/core/count-shaped-producers-refuse-alike.nvst:1000000000000000]
