- **`session.py --wrap` can refuse over a link no session touched, because a rule fragment's
  relative link is resolved from two different places.** A fragment's links are copied verbatim
  into the chapter one directory above it, so `../../../README.md` resolved from
  `docs/rules/packaging/the-banner-states-the-build.md` and escaped the repository from
  `docs/rules/packaging.md` — `git show HEAD:<file>` showed the line unchanged, so "this session's"
  was about which resolver ran rather than about the diff. Write a fragment's links to resolve from
  the *chapter*, `python tools/rules.py --render`, and confirm with `python tools/check-links.py`.
  [until: reviewed 2026-09-07]
