- **A closed dialog's `role=option` rows break every viewer browser test that counts options.** The
  request, stream and hostile tests count `[role=option]` and focus the first `[role=listbox]` on
  the whole page, so a list rendered inside a closed `<dialog>` turned 15 cases into 30-second
  timeouts with no assertion message. Render a dialog's listbox only while it is open, as
  `viewer/src/ui/palette.tsx` does with its `isOpen` signal.
  `until: gone viewer/src/ui/palette.tsx:isOpen`
