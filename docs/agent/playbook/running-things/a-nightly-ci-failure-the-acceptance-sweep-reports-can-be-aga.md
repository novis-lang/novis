- **A nightly CI failure the acceptance sweep reports can be against a remote `main` many commits behind
  the local one.** The cron on `landing` dispatches `.github/workflows/ci.yml` on the remote `main`, and
  the loop never pushes, so most of what it lists is already fixed locally. Compare the run's `headSha`
  with `HEAD` first, and reproduce a failure on the local tree before fixing it.
  `until: gone .github/workflows/ci.yml:dispatches this workflow on`
