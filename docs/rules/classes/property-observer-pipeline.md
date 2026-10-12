Every read or write of a property runs one fixed pipeline, and each interface a class implements adds
its own step to it:

```
Read:   not initialized? → initProperty → get hook or storage → onPropertyGet
Write:  beforePropertySet → set hook or storage                → onPropertySet
```

`initProperty` is `LateInit`'s step (`rule:classes/late-init-initializes-on-the-first-read`),
`beforePropertySet` is `PropertyGuard`'s (`rule:classes/property-guard-runs-before-the-write`), and the
two `on` calls are `PropertyObserver`'s. A class that implements none of them runs only the middle step.

The middle step produces or commits the value exactly as it would with no interface in the picture —
by the property's own hook if it declares one, by plain field storage otherwise. The guard sees the
caller's value before it, and may stop the write by throwing: nothing is stored and no observer runs.
The observer sees the settled value after it. A caller receives what the middle step produced; a slot
holds what it committed, which for a transforming `set` hook is not the caller's own argument. Every
access spelling runs the same pipeline: a write through a `mixed` receiver or through
`Core\Reflect` runs the guard, the hook and the observer the runtime finds on the object's class.

This is a pipeline and not a fallback: a hooked property is not exempt, and a hookless one still
reaches the guard and the observer. A guard or an observer that throws fails the access, propagated
like any other call (`rule:errors/propagation`); an observer's throw arrives after the store.

An element write is a write of the property that holds the array. `$p->tags["a"] = $v`, a compound
assignment, an increment, `??=` and `unset($p->tags["a"])` each run the guard once before the store,
with the key path, and report one `onPropertySet` with the whole array after it, at any depth of the
path, and no `onPropertyGet`: the read of the root that the write makes to separate the array is part
of the write, not an access of its own. Passing the array copies nothing.

Four boundaries follow from writing it about a receiver. An access inside the property's own hooks is
the backing slot and carries no guard and no observer, or one write would be checked and reported
twice. A write that initializes a `lateinit` property while `initProperty` runs is plain in the same
way. A `static` property has no receiving instance and reaches nothing. A guard or an observer that
touches a property of its own class recurses, exactly as any method calling itself does — there is no
re-entry guard, because a guard would be a second rule about which write is the real one.

An observer that is told the committed value has no declared way to see the caller's raw argument
when a `set` hook transformed it. Logic that computes or transforms a value belongs in that
property's own hook (`rule:classes/property-hooks`); the observer only watches the result, and the
guard only allows or stops it.

**Declined:**
- **An element write reports an `onPropertyGet` for its root as well.** One write would then reach
  the observer twice, once as a read nobody wrote.
- **The observer runs only for a property with no hook of its own.** Logging or dirty-tracking every
  write would then be copied into every hook instead of declared once.
- **The observer or the guard may override what a read returns or a write stores.** A second
  authority over a value a hook already decided raises an ordering question with no answer that is
  not arbitrary, and a `void` method removes the question.
- **The guard runs after the `set` hook, on the committed value.** The hook would then have run, and
  any effect it had beyond the slot happened, for a write the guard stops.
