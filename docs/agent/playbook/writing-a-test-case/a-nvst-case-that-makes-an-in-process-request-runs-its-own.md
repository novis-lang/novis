- **A `.nvst` case that makes an in-process request runs its own top-level statements again on the
  far side, so a *generated* key is a different key there.** `Core\Test::request` answers with this
  same program's script frame (`rule:testing/in-process-request`), so a `Core\Crypto::generateKey()`
  above the call draws once in the parent and again in the child, and a link one of them signed never
  verifies for the other — the case then fails as though the member under test were broken. Write a
  fixed key (`Core\Bytes::fill(32, 65)`) for anything a case mints on one side of that call and checks
  on the other. [until: reviewed 2026-09-10]
