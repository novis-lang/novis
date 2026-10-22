Each request that calls an extension gets a **pristine instance** of it, created lazily on first use.
Nothing a guest wrote into a global, a static or its linear memory is there on the next request, and a
test that stores state in one instance must not be able to observe it from the next. No extension
keeps state for a worker's lifetime.

**The instance ends with its request.** In `nvs serve` the instances a request made are dropped when
its connection waits for the next request and nothing else holds the request's tree, or else when its
after-response work ends. A keep-alive connection that waits holds no instance. The guest's budget
reads the tree's stop flags through a weak handle, because the tree keeps the budget, and a strong one
would keep every tree, its instances and its memory for the life of the process.

**Every request in flight can hold an instance of each class it calls.** The pooling allocator has a
fixed number of places, sized once at the first boot: `max_in_flight` times the classes one request
can call, the built-in components and each `[[extension]]` entry. A pool sized below that let a few
slow requests that hold an instance starve every other request that calls an extension. Each place
reserves 6 GiB of address space for its linear memory, which lets compiled guest code skip bounds
checks, so the pool stops at 4096 places, 24 TiB of the 128 TiB a 64-bit process can address. A reload
cannot resize a pool that exists: after a reload that raises `max_in_flight` or adds a class, a call
that needs an instance past the boot's size throws `ExtensionError` until the next boot.

- **Memory:** address space only, reserved at boot; an instance commits memory only as it runs, and
  that memory is charged to its request.

**Declined:**
- **A smaller reservation per place, so the pool fits `max_in_flight` at any size.** Under 4 GiB, every
  guest memory access gains a bounds check, which is latency on the request path bought with address
  space that costs nothing.
- **Growing the pool at a reload.** wasmtime's pool is fixed when its engine is made, and every loaded
  component is compiled into that engine, so a new pool means compiling every extension again.

Instantiation costs about 8 µs with the pooling allocator, paid only for the extensions a request
actually calls. A typical request touches one to three, so the realistic cost is 8–23 µs against a
request budget measured in milliseconds — and the figure is environment-sensitive, so quote it as a
range.

This is also the property that decides tier placement for anything whose defining feature is state
outliving a request. A connection pool, a bound directory session, a broker consumer cannot be a guest,
because a guest re-instantiated per request loses them every time; such a client is Native or nothing
(`rule:core-api/tier-placement`). Cross-request state a program wants is explicit and host-owned
(`rule:concurrency/cross-request-state-is-explicit`).
