Two layers enforce a capability. The **static** one reads the grant tables at compile time, costs
nothing, and answers *may this code ever do this?* The **runtime** one reads the request's effective
configuration at every door and at every optional guard, costs one branch, and answers *may this
request, right now?*

The runtime layer exists because narrowing exists: a request may tighten a capability, and an isolate
may drop grants at the spawn site (`rule:security/script-spawn-capability`). **It may only ever
drop.** There is no runtime grant, and adding one would spend the property this whole design rests on
in exchange for reintroducing the dynamic escape the language closed elsewhere — by having no `eval`
(`rule:security/no-eval`), no string or array callables, and reflection that hands back an inert tree
(`rule:security/reflection-needs-no-capability`).

Static attribution is total in Novis only because those doors are shut. A runtime widen would be a
further one opened, and every claim above it would have to be qualified.
