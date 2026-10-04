The synthesized element type is the canonical union of the element types, **each widened to its base**:
the same widening `rule:types/single-value-types` already performs at a placement, and the same union
canonicalisation the type table uses everywhere. So `['retry' => 1, 'depth' => 3]` yields `array<int>`
and never `array<1|3>`, which would be narrower and would refuse the next write made to it. An enum case
widens to its enum by the same call.

- A nested literal recurses, which is where the whole value of the action is: `array<array<array<float>>>`
  from one keystroke.
- `[]` already has type `array<never>`, which satisfies every `array<T>`, so there is nothing to offer and
  the action does not appear.
- The walk inherits the depth-32 descriptor bound of `rule:types/arrays`: past it the action declines
  rather than proposing a type the checker would then refuse.
- The action offers only a type that renders back to a spelling a developer could have written, because
  the edit it makes is source text a human reads in a diff.

The unit tests are the list: a homogeneous nest at three levels, a heterogeneous literal producing a
canonical union, an integer literal widening to `int`, an enum case widening to its enum, `[]` yielding no
offer, and a literal past depth 32 yielding no offer.
