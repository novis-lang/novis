A Rust test's input is the smallest one that crosses the boundary the test checks, written in terms of
that boundary, and no larger and no smaller. Miri runs a test thousands of times slower than a native
run, so every count past the boundary costs CI minutes and checks nothing, and every count short of it
passes without checking anything at all. Both are wrong, and the right size is found by reading the
code under test, never by scaling the old size.

**How each kind of boundary is crossed:**

- **A limit, cap or ceiling** is crossed by its constant plus one: `MAX_RECURSION_DEPTH as usize + 1`,
  never `200` or `2000`. The constant is named, so the test follows it when the limit moves. A test of
  both sides keeps both: the value at the limit passes and the one past it is refused.
- **A configurable limit** is set small in the test (`ctx.set_memory_limit(..)`, a written config
  key) instead of being filled at its shipped default. One test may keep the default to show that the
  default is what the path reads.
- **A chunk, page, block or buffer** is crossed once, by one unit plus one byte, or as many times as the
  assertion counts. A constant that is already smaller under `cfg!(miri)`, such as `READ_CHUNK`, is
  read where the test sizes its input.
- **Growth, eviction, a sweep or a "stays bounded" claim** runs just past the first point where the
  thing happens, and as many repeats as the assertion needs to tell bounded from unbounded: two or three
  times the bound, not a hundred.
- **A data sweep** (every radix, every width, every codec) runs whole natively. Under Miri it may run a
  sample only when the sample still reaches every path the full sweep reaches: each branch, each edge
  and each kind. The test's comment says which.

**One size, and no `cfg!(miri)` branch, unless the native run needs more.** When the native size is past
the boundary too, it is cut to the same size and the branch goes. A branch stays only where the native
run checks something Miri cannot: a linear-work check that needs a large input to tell O(n) from
O(n²), a real-stack check that must pass a thread's stack, or the full sweep above. Its comment names
that reason.

**The comment names the boundary.** A size carries one line saying which constant or event it crosses.
"A tenth under Miri" or "twice the limit" says that the size was guessed, and the next timeout guesses
again.

**Undershooting is the worse mistake.** A count below its boundary keeps the test green and stops it
checking anything. So a size cut under Miri is derived from the code, and a sample is shown to reach
what the sweep reaches. A test whose boundary is a floor that has no smaller size, such as a password
hash's cost floor or a published vector's length, keeps that size, and if Miri cannot afford it the
test carries `#[cfg_attr(miri, ignore = "<the floor>")]`.

`.config/nextest.toml`'s Miri profile stops a test after five minutes, which catches the worst
overshoot. Nothing mechanical catches an undershoot, so a reviewer reads the size against the code.

- **Cost:** none at run time. A test sized this way runs faster natively and under Miri.

**Declined:**
- **Scaling an old size down by a factor under `cfg!(miri)`.** Every past fix did this. It leaves the
  test either still too large, so the next slow runner times out again, or too small to cross the
  boundary, with nothing to show which.
- **Raising the Miri timeout or adding shards** in place of sizing. It spends CI minutes and leaves the
  waste in every run.
- **Ignoring a slow test under Miri** when a smaller input exists. It drops Miri's check of that path
  entirely.
