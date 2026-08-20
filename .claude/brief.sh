#!/bin/sh
# One call, whole orientation. Prints the parts of this repository an agent reads at the start of
# nearly every session: where the plan stands, what each ADR decided, what the guard tests hold, and
# what actually exists on disk.
#
# It stores no facts of its own. Every line it prints is sliced out of a file it names, so it cannot
# go stale. When a slice comes back empty it says so loudly rather than printing a plausible nothing.
#
# Usage:  sh .claude/brief.sh            # the digest
#         sh .claude/brief.sh --no-git   # skip the working-tree section
#
# This is not a substitute for reading a file you are about to change. It gives you every *decision*;
# the argument behind one still lives in its ADR's Context / Investigation / Alternatives sections,
# which you only need when you intend to overturn it.

set -eu

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

PLAN=docs/implementation-plan.md
ADR_DIR=docs/adr
PROBE=benches/abi-probe

warn() { printf '\n!! brief.sh: %s\n' "$1"; }   # stdout, not stderr: the reader reads stdout

section() { printf '\n\n== %s\n-- source: %s\n\n' "$1" "$2"; }

# ---------------------------------------------------------------- plan status

section "WHERE THE PLAN STANDS" "$PLAN (leading status block)"
status=$(awk '
  NR == 1 { next }
  /^>/    { found = 1; line = $0; sub(/^> ?/, "", line); print line; next }
  found   { exit }
' "$PLAN")
if [ -n "$status" ]; then
  printf '%s\n' "$status"
else
  warn "no status block at the top of $PLAN -- read it directly"
fi

# ------------------------------------------------------- current + next milestone

current=$(printf '%s' "$status" | sed -n 's/.*Milestone \*\*\(M[0-9]\{1,\}\)\*\*.*/\1/p' | head -1)
if [ -n "$current" ]; then
  section "MILESTONE $current, AND THE ONE AFTER IT" "$PLAN (## Milestones)"
  awk -v cur="$current" '
    /^### M[0-9]+ / {
      if ($2 == cur) { printing = 1 }
      else if (printing) { if (++seen > 1) exit }
    }
    printing { print }
  ' "$PLAN"
else
  warn "could not read the current milestone out of the status block -- read $PLAN section Milestones"
fi

# ------------------------------------------------------------------ decisions

section "DECISIONS WITH AN ADR" "$ADR_DIR/*.md (metadata block + In short: everything before ## Context)"
found_adr=0
for adr in "$ADR_DIR"/0*.md; do
  [ -f "$adr" ] || continue
  found_adr=1
  printf -- '---- %s\n' "$adr"
  awk '/^## / { exit } { print }' "$adr"
done
[ "$found_adr" = 1 ] || warn "no ADRs found under $ADR_DIR"

section "DECISIONS WITH NO ADR (titles only)" "$ADR_DIR/README.md section Decisions taken at project start"
lead=$(awk '
  /^## Decisions taken at project start/ { inside = 1; next }
  inside && /^## /                       { exit }
  inside && /^\*\*/ {
    line = $0
    sub(/^\*\*/, "", line)
    sub(/\*\*.*$/, "", line)
    print "- " line
  }
' "$ADR_DIR/README.md")
if [ -n "$lead" ]; then
  printf '%s\n' "$lead"
  printf '\n(each is one paragraph in that section: open it for the reasoning)\n'
else
  warn "could not slice the project-start decisions out of $ADR_DIR/README.md"
fi

# --------------------------------------------------------------- guard tests

section "WHAT IS ACTUALLY GUARDED" "$PROBE/tests/*.rs -- authoritative for every measured number"
printf 'A test name is the claim; a bracketed threshold is the bound it holds. If one of these fails,\n'
printf 'the ADR naming it needs revisiting -- not the threshold.\n'
guards=$(for t in "$PROBE"/tests/*.rs; do
  [ -f "$t" ] || continue
  printf '\n%s\n' "$t"
  awk '
    # a feature gate applies to the mod that follows it, or to the next test alone
    /^ *#\[cfg\(feature = "/ {
      match($0, /"[^"]+"/); pending_feat = substr($0, RSTART, RLENGTH); next
    }
    pending_feat != "" && /^ *mod / { mod_feat = pending_feat; pending_feat = ""; next }
    /^ *#\[test\]/ { pending = 1; next }
    pending && /^ *fn [a-z_0-9]+\(\)/ {
      match($0, /fn [a-z_0-9]+/); name = substr($0, RSTART + 3, RLENGTH - 3)
      order[++n] = name; pending = 0
      feat = pending_feat != "" ? pending_feat : mod_feat
      pending_feat = ""
      if (feat != "") gate[name] = feat
      cur = name; next
    }
    cur && /const (MAX|MIN)[A-Z_]*: *f64 *=/ {
      line = $0; sub(/^ *const /, "", line); sub(/;.*$/, "", line)
      bound[cur] = bound[cur] (bound[cur] ? ", " : "") line
    }
    /^}/ { cur = ""; mod_feat = "" }   # a column-zero brace closes a gated mod as well as a fn
    END {
      for (i = 1; i <= n; i++) {
        name = order[i]
        printf "  %s%s%s\n", name,
          (bound[name] ? "  [" bound[name] "]" : ""),
          (gate[name] ? "  (feature " gate[name] ")" : "")
      }
    }
  ' "$t"
done)
if [ -n "$guards" ]; then
  printf '%s\n' "$guards"
else
  warn "found no guard tests under $PROBE/tests -- that directory is the source of truth, check it"
fi

benches=$(ls "$PROBE"/benches/*.rs 2>/dev/null | sed 's/^/  /' || true)
if [ -n "$benches" ]; then
  printf '\nbenchmarks (unguarded, for tracking figures by hand):\n%s\n' "$benches"
fi

# -------------------------------------------------------------- what exists

section "WHAT EXISTS ON DISK" "the filesystem, not the plan"
printf 'crates:  %s\n' "$(ls crates 2>/dev/null | tr '\n' ' ')"
printf 'benches: %s\n' "$(ls benches 2>/dev/null | tr '\n' ' ')"
printf 'docs:    %s\n' "$(ls docs 2>/dev/null | tr '\n' ' ')"
if [ -d docs/spec ] && [ -n "$(ls -A docs/spec 2>/dev/null)" ]; then
  printf 'spec:    %s\n' "$(ls docs/spec | tr '\n' ' ')"
else
  printf 'spec:    unwritten -- say so rather than inferring language semantics\n'
fi

# ------------------------------------------------------------- working tree

if [ "${1:-}" != "--no-git" ] && [ -d .git ]; then
  section "WORKING TREE" "git"
  printf 'branch:  %s\n' "$(git rev-parse --abbrev-ref HEAD)"
  printf 'HEAD:    %s\n' "$(git log -1 --format='%h %s')"
  changed=$(git status --porcelain)
  if [ -n "$changed" ]; then
    printf 'changed:\n%s\n' "$(printf '%s\n' "$changed" | sed 's/^/  /')"
  else
    printf 'changed: nothing\n'
  fi
fi
