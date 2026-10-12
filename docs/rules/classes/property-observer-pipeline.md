Every read or write of a property on a class implementing `PropertyObserver` runs two steps in one
fixed order. First the value is produced or committed exactly as it would be without the interface in
the picture — by the property's own hook if it declares one, by plain field storage otherwise. Then
the settled value is passed to `onPropertyGet` or `onPropertySet`. A caller receives what the first
step produced; a slot holds what the first step committed, which for a transforming `set` hook is not
the caller's own argument.

This is a pipeline and not a fallback: a hooked property is not exempt, and a hookless one still
reaches the observer. An observer that throws still fails the access, propagated like any other call
(`rule:errors/propagation`).

An element write is a write of the property that holds the array. `$p->tags["a"] = $v`, a compound
assignment, an increment, `??=` and `unset($p->tags["a"])` each report one `onPropertySet` with the
whole array after it is stored, at any depth of the path, and no `onPropertyGet`: the read of the
root that the write makes to separate the array is part of the write, not an access of its own.
Passing the array copies nothing.

Three boundaries follow from writing it about a receiver. An access inside the property's own hooks
is the backing slot and carries no observer, or one write would be reported twice. A `static`
property has no receiving instance and reaches nothing. An observer that touches a property of its own
class recurses, exactly as any method calling itself does — there is no re-entry guard, because a
guard would be a second rule about which write is the real one.

An observer that is told the committed value has no declared way to see the caller's raw argument
when a `set` hook transformed it. Logic that computes or transforms a value belongs in that
property's own hook (`rule:classes/property-hooks`); the observer only watches the result.

**Declined:**
- **An element write reports an `onPropertyGet` for its root as well.** One write would then reach
  the observer twice, once as a read nobody wrote.
- **The observer runs only for a property with no hook of its own.** Logging or dirty-tracking every
  write would then be copied into every hook instead of declared once.
- **The observer may override what a read returns or a write stores.** A second authority over a
  value a hook already decided raises an ordering question with no answer that is not arbitrary, and
  a `void` observer removes the question.
