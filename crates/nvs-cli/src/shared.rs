//! `nvs test --shared-worker`'s worker: one thread that runs many `.nvst`
//! cases' programs in turn, each in an isolate of its own
//! (`rule:security/isolate-shares-nothing`).
//!
//! `nvs_test` runs every case fresh, in a process of its own, and then hands
//! an eligible case's program to [`nvs_test::SharedWorker::run`]. Here the programs share what
//! the requests of one served core share: the thread, its scheduler and
//! reactor, the unit cache and every process-wide table. A case whose output
//! differs from its fresh run read something an earlier case left behind on
//! the worker.
//!
//! # What a case runs under
//!
//! Each case gets a parent context of its own, as each served request gets a
//! root of its own, built after the case's compile so that the unit cache is
//! not charged to the case's memory limit. The configuration is the tree a
//! fresh run of the first case resolves: every case's directory is a sibling
//! under the runner's root and holds nothing but `case.nvs`, so that tree is
//! every case's. The parent carries the snapshot and its origin, which is what
//! [`nvs_runtime::Ctx::isolate`] hands a child. The child does not inherit the
//! program name or the command line, so a case that reads either answers
//! differently here, and `nvs_test::run`'s `shares_a_worker` is what keeps
//! such cases fresh only.
//!
//! **What it spends:** the shared [`Compiler`] keeps every case's unit for the
//! life of the run, so the worker holds O(cases) compiled code until `nvs test`
//! exits. Each case's directory is a new path, so no unit is reused. This is a
//! test run's cost, and no served process holds it.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;

use nvs_runtime::script::Resolver as _;

use crate::config;
use crate::script::Compiler;

/// One case's program sent to the worker, and where its answer goes.
type Job = (PathBuf, mpsc::Sender<Result<String, String>>);

/// The handle `nvs_test` calls from its own threads. Every call queues behind
/// the ones before it, because one thread runs them all.
#[derive(Debug)]
pub(crate) struct Worker {
    /// The worker thread's queue. Behind a lock so that the handle is `Sync`
    /// on every toolchain, and the lock is held only for the send.
    jobs: Mutex<mpsc::Sender<Job>>,
}

impl Worker {
    /// Starts the worker thread. It ends when this handle is dropped and the
    /// last queued case has been answered.
    ///
    /// `init` is the project's init gate, passed on to the configuration tree
    /// the first case resolves.
    ///
    /// # Errors
    ///
    /// The message to print when the thread cannot be started.
    pub(crate) fn start(init: config::Init) -> Result<Self, String> {
        let (jobs, queue) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("nvs-shared-worker".to_owned())
            .spawn(move || serve(&queue, init))
            .map_err(|error| format!("could not start the shared worker: {error}"))?;
        Ok(Self {
            jobs: Mutex::new(jobs),
        })
    }
}

impl nvs_test::SharedWorker for Worker {
    fn run(&self, path: &Path) -> Result<String, String> {
        let (reply, answer) = mpsc::channel();
        self.jobs
            .lock()
            .map_err(|_| "the shared worker's queue is poisoned".to_owned())?
            .send((path.to_path_buf(), reply))
            .map_err(|_| "the shared worker has stopped".to_owned())?;
        answer
            .recv_timeout(nvs_test::run::CASE_TIMEOUT)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => format!(
                    "did not finish within {} s on the shared worker",
                    nvs_test::run::CASE_TIMEOUT.as_secs()
                ),
                mpsc::RecvTimeoutError::Disconnected => {
                    "the shared worker stopped while running this case".to_owned()
                }
            })?
    }
}

/// The worker thread: what a served core holds, built on the first case and
/// kept for every case after it.
struct Serving {
    compiler: Compiler,
    sched: nvs_host::Scheduler,
    /// The configuration every case's parent context is handed.
    snapshot: std::sync::Arc<nvs_config::Snapshot>,
    _reactor: nvs_host::reactor::Installed,
    _calling: nvs_runtime::extension::Installed,
}

/// Answers every case the queue delivers, in order.
fn serve(queue: &mpsc::Receiver<Job>, init: config::Init) {
    let mut serving: Option<Result<Serving, String>> = None;
    for (path, reply) in queue {
        let answer = match serving.get_or_insert_with(|| Serving::boot(&path, init)) {
            Ok(serving) => serving.run(&path),
            Err(error) => Err(error.clone()),
        };
        // A case that gave up waiting has dropped its receiver, and that is
        // already its failure.
        let _ = reply.send(answer);
    }
}

impl Serving {
    /// The worker's state, under the tree a fresh run of `first` resolves.
    fn boot(first: &Path, init: config::Init) -> Result<Self, String> {
        let dir = first
            .parent()
            .ok_or_else(|| format!("{} has no directory", first.display()))?;
        let mut sources = nvs_diagnostics::SourceMap::new();
        let (snapshot, origins) = config::boot_in(
            dir,
            nvs_config::data::config_file().as_deref(),
            &[],
            Some(first),
            &mut sources,
            init,
        )
        .map_err(|diagnostic| {
            format!(
                "the configuration tree does not resolve: {}",
                diagnostic.message
            )
        })?;
        // The process's one outbound TLS client, built as `nvs run` builds it.
        // A client already built is an earlier worker's in this process, from
        // the same tree, so it is kept.
        match nvs_host::tls::configure(&config::policy_of(&snapshot)) {
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                return Err(format!("could not build the outbound TLS client: {error}"));
            }
            _ => {}
        }
        // The built-in components and every `[[extension]]` the tree names,
        // loaded once for the worker's life as `nvs run` loads them for its
        // one program.
        let calls = crate::extensions::loaded(&snapshot.config, &origins, &sources)
            .map_err(|diagnostic| diagnostic.message)?;
        let calling =
            nvs_runtime::extension::install(std::rc::Rc::new(crate::extensions::Calls::new(calls)));
        let compiler = Compiler::new(&snapshot.config);
        let reactor = nvs_host::Reactor::new()
            .map_err(|error| format!("could not start the reactor: {error}"))?;
        Ok(Self {
            compiler,
            sched: nvs_host::Scheduler::new(),
            snapshot,
            _reactor: nvs_host::reactor::install(reactor),
            _calling: calling,
        })
    }

    /// Runs the program at `path` in an isolate under a parent context of its
    /// own, and returns what it wrote.
    fn run(&mut self, path: &Path) -> Result<String, String> {
        let program = self.compiler.resolve(&path.to_string_lossy())?;
        let mut parent = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
        if let Some(origin) = self.snapshot.origin.clone() {
            parent.set_origin(&origin);
        }
        parent.set_config(std::sync::Arc::clone(&self.snapshot));
        // Each case's app folder is its own, as a fresh run of it has.
        if let Ok(canonical) = nvs_config::resolve::Files::canonical(&config::LocalFiles, path) {
            let folder = nvs_config::app::folder(&canonical, &self.snapshot.blocks);
            parent.set_app_folder(folder.into());
        }
        let answered: std::rc::Rc<std::cell::RefCell<Option<Result<String, String>>>> =
            std::rc::Rc::default();
        self.sched.spawn(parent, nvs_runtime::TaskRoot::Request, {
            let answered = std::rc::Rc::clone(&answered);
            move |ctx| {
                let completion = nvs_host::Isolate::new(
                    program,
                    nvs_runtime::Value::null(),
                    nvs_host::Output::Capture,
                )
                .run(ctx);
                *answered.borrow_mut() = Some(match completion {
                    Ok(mut completion) => {
                        completion.discard_value();
                        match completion.error {
                            Some(failure) if !completion.ok => {
                                Err(format!("{}: {}", failure.class, failure.message))
                            }
                            None if !completion.ok => {
                                Err("the program failed and named no failure".to_owned())
                            }
                            _ => Ok(String::from_utf8_lossy(&completion.output).into_owned()),
                        }
                    }
                    Err(error) => Err(error.to_string()),
                });
            }
        });
        let ran = nvs_runtime::script::scoped(&self.compiler, || {
            nvs_host::run_until_idle(&mut self.sched)
        });
        // The parent contexts end here, with their cases.
        drop(self.sched.take_finished());
        ran.map_err(|error| format!("the scheduler stopped: {error}"))?;
        answered
            .borrow_mut()
            .take()
            .unwrap_or_else(|| Err("the case's task ran nothing".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use nvs_test::SharedWorker as _;

    use super::Worker;
    use crate::config::Init;

    /// One program run twice on one worker shares its compiled unit, and the
    /// second run still reads the static's declared value, not what the first
    /// run wrote (`rule:statements/an-isolate-has-its-own-statics`).
    #[test]
    fn a_static_one_case_writes_is_not_seen_by_the_next_case_on_the_worker() {
        let dir = nvs_repo::scratch("shared-worker-statics");
        let path = dir.join("case.nvs");
        std::fs::write(
            &path,
            "<?nvs\nclass Seen {\n    public static int $count = 0;\n}\nSeen::$count += 1;\necho Seen::$count, \"\\n\";\n",
        )
        .expect("the case is writable");
        let worker = Worker::start(Init::Never).expect("the worker starts");

        let first = worker.run(&path);
        let second = worker.run(&path);

        assert_eq!(first.as_deref(), Ok("1\n"), "the first run counts once");
        assert_eq!(
            second.as_deref(),
            Ok("1\n"),
            "the second run starts from the declared value"
        );
    }

    /// A throw on the worker is the case's failure, with the class and the
    /// message, and the worker goes on to answer the next case.
    #[test]
    fn a_throw_on_the_worker_fails_that_case_and_the_next_one_still_runs() {
        let dir = nvs_repo::scratch("shared-worker-throw");
        let throwing = dir.join("throws.nvs");
        std::fs::write(&throwing, "<?nvs\nthrow new RuntimeError(\"no\");\n")
            .expect("the case is writable");
        let echoing = dir.join("echoes.nvs");
        std::fs::write(&echoing, "<?nvs\necho \"yes\";\n").expect("the case is writable");
        let worker = Worker::start(Init::Never).expect("the worker starts");

        let thrown = worker.run(&throwing).expect_err("the throw fails the case");
        assert!(
            thrown.contains("RuntimeError") && thrown.contains("no"),
            "the failure names the class and the message: {thrown}"
        );
        assert_eq!(worker.run(&echoing).as_deref(), Ok("yes"));
    }
}
