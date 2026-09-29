Returns how far a time zone is from UTC at one instant.

The result is a `Duration` of whole seconds. It is positive for a zone east of UTC, negative for a
zone west of UTC, and zero for UTC itself. For example, `Europe/Berlin` returns one hour in winter.

You give an instant because many zones have more than one offset. A zone with summer time moves its
clocks forward in spring and back in autumn. `Europe/Berlin` is one hour ahead of UTC in January
and two hours ahead in July. The same zone can also have a different offset in the past, when the
region changed its rules. A zone made with `Zone::fixed` has the same offset at every instant.

The examples show a zone in winter and in summer, the second when the clocks change, and the
difference in hours between two offices.
