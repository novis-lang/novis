// The viewer's frame: the header and the region each view renders into. The views themselves are
// added by `docs/goals/debug-viewer.md`'s later stages.

export function App() {
  return (
    <>
      <header class="bar">
        <h1>Novis debug</h1>
      </header>
      <main id="view" aria-live="polite" />
    </>
  );
}
