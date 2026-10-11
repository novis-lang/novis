An **encrypted document throws** a `ParseError` naming the encryption. There is no password option,
and decrypting stays in the third-party channel. An **embedded object the engine recognises but cannot
decode throws** naming the object, rather than rendering a blank region: a generated image must be
deterministic, not approximately right, and a silently incomplete raster is a repair
(`rule:errors/ambiguous-input-refused`). An external reference never resolves, because nothing in the
guest can resolve one, and JavaScript, launch actions and embedded files are inert by construction,
since rasterising reads a page's appearance and never executes an action.

Text extraction, metadata, attachments, and splitting and merging pages are **different jobs over the
same bytes**, and are `Novis\Pdf`'s (`rule:core-classes/pdf-reader-and-merge`), which refuses an
encrypted source the same way. Form filling, signing and password decryption are no component's, and
belong to the third-party channel. A document the interpreter renders wrong has no first-party
fallback.

The surface this leaves open is an *approximated* construct — one the interpreter supports but
renders imperfectly — which is a silent fidelity miss rather than a detectable omission, and is owned
as a known consequence rather than closed by a rule.

The decoder's fixtures hold this inside the guest, and owe:
- an encrypted fixture that throws `ParseError` naming the encryption, and a fixture embedding an
  object the engine cannot decode that throws naming it;
- a fixture carrying an external reference, a JavaScript action and an embedded file that rasterises
  with nothing resolved, nothing executed and no network or filesystem access observable;
- a **bomb corpus** — recursive form XObjects, an xref loop, a stream-length lie and a deflate bomb —
  where each case ends in a throw or a contained trap under the epoch deadline, never a hang.

**Not shipped.** There is no PDF decoder in the image component.
