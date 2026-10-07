Before any pixel buffer is allocated, the component reads the **declared** dimensions — width times
height times frames for an animated input, the declared canvas for an SVG, the selected page's box at
the requested resolution for a PDF — and refuses an image over `[image] max_pixels` with a throw
naming the cap and the declared size. A decompression bomb is refused from what the file claims,
never discovered by exhausting memory.

It is a throw rather than a resource-limit fatal, because a rejected upload is an ordinary outcome
the application answers with a status code. A call may pass its own cap **only to lower** it, the
same monotone rule extension manifests take.

Bytes that are not an image the roster decodes throw a parse error; a codec fault beyond that is a
trap the sandbox contains, and it throws `ExtensionError` (`rule:packaging/a-guest-crash-throws`).

What it spends, per request that calls the component: the encoded input, at most two frames at the
cap — source and result, 192 MB worst case at RGBA8 — and the encoded output, all charged to the
calling request's memory cap (`rule:packaging/a-guest-runs-under-the-requests-budget`), and none of
which outlives the request. There are no threads in a guest, so a bulk import parallelises across
coroutines and cores rather than inside one call.

The cap is `[image] max_pixels`, the block's only key. It is `System` and reloads, and with nothing
written it is `"24M"`, read in the size grammar every other key of the file uses, so `M` is 2²⁰
pixels. `false` and zero are refused where they are written, because neither is a cap.

**Not shipped.** The key is on disk (`nvs_config::image`, `crates/nvs-config/tests/directives.rs`),
and the image component's `run` checks a header against a cap before it decodes
(`crates/nvs-ext/tests/image_decode.rs`). The host sets every encoded source's `maxPixels` to the
smaller of the call's and the key's value in the request's snapshot before the call crosses
(`nvs_ext::builtin::cap_pixels`), so a call lowers the cap and never raises it
(`tests/conformance/novis/image-a-bomb-is-refused-before-it-is-decoded.nvst`). The
`Novis\Image` builder that would pass a program's own `{maxPixels}` is not written.
