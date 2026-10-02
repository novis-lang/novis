# Sourced, never run: empties `$CARGO_TARGET_DIR` when a different checkout built it.
#
# A WSL leg keeps its target directory under /var/tmp, outside the checkout, so it outlives a move
# of the checkout. Cargo then reuses every artefact: a workspace crate's fingerprint names its path
# relative to the workspace, and `env!("CARGO_MANIFEST_DIR")` is not part of it. A test binary
# built before the move still resolves the repository root to the old path and panics on the first
# file it opens. `nvs-repo` is the crate that reads it.
#
# The stamp holds the checkout that filled the directory. A directory with no stamp, or with
# another checkout in it, is cleaned once and rebuilt cold. The caller has already `cd`ed to the
# repository root and exported `CARGO_TARGET_DIR`.
stamp="$CARGO_TARGET_DIR/.nvs-checkout"
if [ -d "$CARGO_TARGET_DIR" ] && [ "$(cat "$stamp" 2>/dev/null)" != "$PWD" ]; then
    echo "target-checkout: $CARGO_TARGET_DIR was built from another checkout, cleaning it" >&2
    cargo clean --quiet || exit 1
fi
mkdir -p "$CARGO_TARGET_DIR" && echo "$PWD" > "$stamp"
