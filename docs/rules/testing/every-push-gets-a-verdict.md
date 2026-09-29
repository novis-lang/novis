In-progress runs are cancelled for pull requests only. A run on `main` is never
cancelled by the next commit, so every commit there carries its own result — which is what a bisect
reads, and what a loop committing one slice at a time needs.

The concurrency group is keyed by event as well, so a long nightly never sits in front of a short
push.
