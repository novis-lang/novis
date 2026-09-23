- **A `dossier.py --run hostile` sweep started while `verify.py --start` is still running fails
  attacks that pass alone, as `still running after Ns -- unbounded`.** The build and the test
  binaries take the cores the attacks' time limits were set against, so heavy attacks
  (`Core\Html::sanitize`, `Core\Http\Response::jsonAs`, `Core\IO::append`) miss their limits and the
  report looks like a regression across unrelated members. Run the sweep after `verify.py --wait`
  returns; only the cases that failed run again, because the green ones are cached.
  [until: reviewed 2026-09-23]
