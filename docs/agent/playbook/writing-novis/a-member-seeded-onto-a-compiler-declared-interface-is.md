- **A member seeded onto a compiler-declared interface is invisible to `nvs_hir::members`.**
  `member_declared_rec` (`crates/nvs-hir/src/members.rs:1307`) walks `implements` into `Parses`, finds
  no `MemberTable` entry — nothing declares a reserved interface in source — so `Slug::tryParse($s)` is
  `E0309` even though `crate::conformance` agrees the class inherits it. Do not repair that by trusting
  the roster the way line 1315 trusts `Core`: while the default body has no compiled function, the
  refusal is the safe answer and the alternative is a dispatch to nothing.
  [until: reviewed 2026-09-08]
