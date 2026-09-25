- **A `## Next group` item's rationale about PHP behaviour is a hypothesis, not a specification, and
  one `php -r` settles it.** An item claiming "`finally` must still run on `exit`, which makes this
  an unwind" framed a fourth unwind kind, while `php -r 'try { exit(3); } finally { echo "f"; }'`
  prints nothing and exits 3, so the cheap shape — a helper whose success is a non-`OK` status — was
  the PHP-exact one. Priority 2 decides these and PHP is on `PATH`: run the twin *before* costing
  the design. [until: gone AGENTS.md:PHP-compatible observable behaviour]
