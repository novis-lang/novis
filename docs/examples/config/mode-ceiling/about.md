The most permissive mode any code on this host may select.

Novis has two modes and no third: production, which a host runs in unless something says otherwise,
and development, which trades the safe defaults away for the detail a developer wants. A program may
flip its own mode — one application on a shared host can need the other one — and this key says how
far that flip may go.

Written nowhere it equals the mode the host started in. A production machine is then unreachable
from development mode with nothing configured at all, and a developer who started their server in
development mode flips freely without writing anything either; only a host running both kinds of
application at once needs the line.

Past the ceiling the flip answers false and leaves the mode where it was. Raising the ceiling itself
is the operator's alone, because a program that could raise it could grant itself the mode it had
just been refused.

The example flips in both directions and shows which of the two calls is turned away.
