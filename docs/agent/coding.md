# Code that holds everywhere

CI runs every change on Linux, macOS, aarch64 and a fresh Windows runner, under load, under ASAN and
Miri, and with the cost guards. The dev machine runs it on Windows alone. Each rule below is a mistake
in product code that passed here and failed there, and each failure cost a CI round of an hour or two.
How a test is written is [testing.md](testing.md) § *A test holds on every CI runner*.

- **State lives exactly as long as the unit it belongs to.** What belongs to one request, one test case
  or one task is made with it and ends with it, never at worker, core or process scope. A shared
  worker kept one extension-call host for its whole life, so every case after the first read its CPU
  budget from a request tree that was gone. This is *Security and request isolation*, not a detail.
- **Every way out releases what the normal way out releases.** An error edge, a limit or cancel stop,
  an early return and a safepoint are ways out of a frame or a request. A loop's back-edge poll
  returned without releasing the frame's locals, so each request stopped by its memory limit kept its
  whole budget for the life of the process. A new way out gets a test that the memory comes back.
- **When a second path can end the same thing, every reader of the first path's signal reads both.**
  Find the readers with Grep before the second path lands. A sleep that ended at the deadline claimed
  the stop itself, and the test runner, which read only the clock's word, named the wrong limit.
- **A file, a database or a port is opened by two workers at once.** Handle the busy answer with a
  bounded retry, and test two opens together. Two SQLite opens raced to switch one file to WAL, and
  one failed with `database is locked`.
- **Nothing on the request path is made on its first use.** Build it when its owner is built. On
  macOS a `std::sync::Mutex` allocates on its first lock, so a queue first locked inside a request
  charged that request 64 bytes.
- **A mechanism costs nothing where it is not used.** Turn it on for the class, function or request
  that uses it, never for its neighbours. A class with one hooked property put a tag test on every
  read of its plain ones. When the change touches a mechanism a cost guard covers, run the guards
  once in the gate stage: `cargo test --release -p nvs-abi-probe`.
- **A check judges the text it was given before any step whose meaning depends on the platform.**
  Joining, canonicalising and splitting a path are such steps: a backslash separates on Windows
  alone, so `\\host\share` joined to a base passed a check on Linux. Compare canonical paths, and test
  a path behind a link: Ubuntu's `/bin` holds links into another folder.
- **A new check that refuses a setup is tried against every setup that runs it**: the dev checkout,
  CI's runners, a container, a read-only mount. Name them in the commit. The ownership check refused
  every test tree on CI's Windows runner, whose drive grants `BUILTIN\Users` the right to create files.
- **A change to paths, permissions, sockets, processes or signals runs its tests on Linux before its
  commit:** `bun nv linux-test --test <name> <filter>`, minutes once its build is warm. macOS has no
  local run, so a macOS difference is CI's.
