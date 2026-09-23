- **A bench over a member that returns a scalar can still measure allocations, so `// bench:
  allocations 0` does not follow from the return type.** `benches/members/README.md` says a member
  returning a scalar declares it, but the counts include the allocator's own per-thread totals, and
  `Core\Encoding::isValidText` — a `bool` answer over a decode it performs and discards — measures
  0.50 allocations per operation. Read the member's own doc comment for what it does inside before
  declaring a count, and leave the declaration off where the member allocates on purpose.
  [until: reviewed 2026-09-22]
