Spawning an isolate asks `script.spawn`, a grant naming the roots that code may be executed *from*.
Being able to read a file is not permission to run it, so `fs.read` does not imply `script.spawn`:
the two answer different questions, and a template directory that is readable by design should not
become an execution root by accident.

**With the key not written, a script in the app folder spawns**, on a CLI run and in server work,
from a `spawn script` door and from a `[[schedule]]` entry alike. `require` already runs that code,
so spawning it grants nothing new. The default leaves out every path with a component below the app
folder that starts with `.` and every file named `nvs.toml`, as the default read does
(`rule:security/capability-question-is-grant-and-scope`). It also leaves out every directory
`Core\IO::temporaryDir` made for this run, so a run never spawns a script it wrote, even where the
temporary root is inside the app folder. The method form (`Class::method`) names code already in the
running unit, so it spawns too. A script outside the app folder needs a written `spawn` root, and a
written list replaces the default whole.

The entry path is canonicalised and then prefix-checked against the granted roots
(`rule:security/path-scope-canonicalise-then-prefix`), which closes traversal by construction rather
than by validation. A dynamic path is allowed — a queue worker needs one — but it can only ever land
inside the app folder or a root an operator wrote down.

The child's grants are the parent's effective grants, optionally narrowed at the spawn site. Nothing
widens: a parent that has dropped `net.connect` cannot regain it by spawning
(`rule:security/no-runtime-grant`).

Declined:

- **A spawn default that covers the run's own temporary directories.** Server work writes there by
  default, so a request could write a script and then run it.
