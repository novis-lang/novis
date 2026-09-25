- **Two rules can describe one wire event, and the one you are handed may state the answer while the
  other states the precondition.** `rule:http-server/cors-is-closed-until-origins-are-named` says
  what a preflight is answered with, but `rule:http-server/a-request-resolves-in-five-steps` defines
  one as `OPTIONS` carrying `Origin` and `Access-Control-Request-Method`, and `Cors::preflight` read
  only the second header — invisible while the policy was closed. Grep `docs/rules` for the noun
  before writing the predicate. [until: gone crates/nvs-server/src/cors.rs:fn preflight]
