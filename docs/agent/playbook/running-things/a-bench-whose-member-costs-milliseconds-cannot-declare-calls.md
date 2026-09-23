- **A bench whose member costs milliseconds cannot declare `calls 0`.** Every declaration is met to
  within a hundredth per operation, and the one `Bench::run` frame is one divided by the iteration
  count — nothing at a few hundred thousand rounds, and well past the tolerance once the count has to
  stay tiny for the program to finish in under a second. Declare only `iterations` there;
  `benches/members/README.md` § *What a bench declares* is the rule. [until: reviewed 2026-09-21]
