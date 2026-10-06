//! `Core\Os` — the five facts a program may ask about the host it is running
//! on, replacing `posix_*` minus fork, `php_uname`, `getrusage` and
//! `sys_getloadavg`.
//!
//! `rule:core-api/tier-roster` places the class
//! ([0051](/docs/decisions/0051.md) § 3) and the spec's § 16 row is the member
//! list: `pid`, `hostname`, `cpuCount`, `residentBytes`, `loadAverage`. One
//! member per fact and never one string or one array to take apart, which is
//! `php_uname`'s and `getrusage`'s migration rows both.
//!
//! # This class is not a door, so it declares no capability
//!
//! Every answer here is a fact about the process doing the asking: its own
//! number, the name of the host it already runs on, the cores it may already
//! use, the memory it already holds, and how busy the machine it is already on
//! is. A program learns nothing about anything it does not inhabit, so there is
//! nothing to grant and `Core\Os` has **no row** in
//! [`crate::registry::CAPABILITIES`] — `Core\Path`'s position, not
//! `Core\RateLimit::shed`'s. A `None` row says a member *of a door* reaches
//! nothing, and a class becomes a door in that table's reading the moment any
//! row names it, so five `None` rows here would make this class look like one.
//!
//! The syscalls themselves are [`nvs_runtime::os`]'s, because ADR 0118 § 2
//! forbids this crate from reaching the operating system itself and
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` holds it to that.
//!
//! # Why `memoryUsage` is not here
//!
//! [0148](/docs/decisions/0148.md) §§ 11-12 split PHP's one ambiguous question
//! in two. A **request's** held bytes are `Core\Budget::memoryHeld`, measured
//! against `memoryLimit` and remembered as `memoryPeak`; the **process's**
//! resident set is [`residentBytes`](nvs_runtime::os::resident_bytes) and stays
//! here, where a host fact belongs. A member among these called `memoryUsage`
//! would be read as the process's by everyone who had not read that record.
//!
//! # Why `loadAverage` throws rather than answering zero
//!
//! Windows keeps no load average, and the processor-time percentage suggested
//! in its place answers a different question. Answering `[0.0, 0.0, 0.0]` there
//! would be a number a program could act on, so the member says the platform
//! has no answer and names it — the same shape as every other member that
//! cannot do on one platform what it does on another.

use nvs_runtime::{Fault, NvsArray, NvsStr, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Os";

/// `Core\Os`'s registry rows — the spec's § 16 row, whole and in its order.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "pid",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_os_pid",
            doc: Some(&PID_DOC),
        },
        CoreMethod {
            name: "hostname",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_os_hostname",
            doc: Some(&HOSTNAME_DOC),
        },
        CoreMethod {
            name: "cpuCount",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_os_cpu_count",
            doc: Some(&CPU_COUNT_DOC),
        },
        CoreMethod {
            name: "residentBytes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_os_resident_bytes",
            doc: Some(&RESIDENT_BYTES_DOC),
        },
        CoreMethod {
            name: "loadAverage",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Float),
            symbol: "nvs_core_os_load_average",
            doc: Some(&LOAD_AVERAGE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Os`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Facts about the machine your program runs on and about the program itself. `pid` \
            returns the process number, `hostname` the machine's name and `cpuCount` how many \
            cores the program may use. `residentBytes` returns the memory of the whole process, \
            and `loadAverage` how busy the machine is. No grant is needed.",
};

/// `Core\Os::pid`'s reference card — `rule:core-api/reference-card`.
const PID_DOC: MethodDoc = MethodDoc {
    short: "Returns the number the operating system gives this process. The number belongs to \
            the whole process. Two requests served by the same process get the same number.",
    params: &[],
    ret: "The process number, which is larger than 0.",
    errors: &[],
};

/// `Core\Os::hostname`'s reference card — `rule:core-api/reference-card`.
const HOSTNAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the machine this program runs on. It reads the name the machine \
            has for itself, and it does not use the network.",
    params: &[],
    ret: "The machine's name.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The operating system did not return a name, or the name is not valid UTF-8.",
    }],
};

/// `Core\Os::cpuCount`'s reference card — `rule:core-api/reference-card`.
const CPU_COUNT_DOC: MethodDoc = MethodDoc {
    short: "Returns how many CPU cores this program may use. A server or a container can limit a \
            program to some of its cores. A program in a container with 2 cores on a machine with \
            96 cores gets 2. The web server starts this same number of workers.",
    params: &[],
    ret: "The number of cores. It is always 1 or more.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The operating system did not say how many cores this program may use.",
    }],
};

/// `Core\Os::residentBytes`'s reference card — `rule:core-api/reference-card`.
const RESIDENT_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the most memory this process has used so far, in bytes. On Windows it is \
            the peak working set. The \
            number covers every request the process has served, so it never gets smaller. For \
            the memory of one request, use `Core\\Budget`.",
    params: &[],
    ret: "The most memory the process has used, in bytes.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The operating system did not report the memory of this process.",
    }],
};

/// `Core\Os::loadAverage`'s reference card — `rule:core-api/reference-card`.
const LOAD_AVERAGE_DOC: MethodDoc = MethodDoc {
    short: "Returns how busy the machine is, as three numbers: the averages over the last 1, 5 \
            and 15 minutes. Each number counts the \
            processes that are running or waiting to run. A number larger than `cpuCount()` \
            means some processes are waiting.",
    params: &[],
    ret: "Three floats: the averages over 1, 5 and 15 minutes, in that order.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "Windows does not keep a load average, so this method throws there. The message \
               names the platform.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_os_pid" => (nvs_core_os_pid as *const ()).cast(),
        "nvs_core_os_hostname" => (nvs_core_os_hostname as *const ()).cast(),
        "nvs_core_os_cpu_count" => (nvs_core_os_cpu_count as *const ()).cast(),
        "nvs_core_os_resident_bytes" => (nvs_core_os_resident_bytes as *const ()).cast(),
        "nvs_core_os_load_average" => (nvs_core_os_load_average as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Os::pid(): uint` — replacing `getmypid`.
    ///
    /// The one member here that cannot fail: a process always has a number, and
    /// `std` already answers it without a platform call.
    fn nvs_core_os_pid(_ctx, _args: [0]) {
        Ok(Value::uint(u64::from(nvs_runtime::os::pid())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Os::hostname(): string` — replacing `gethostname` and the field of
    /// `php_uname` a program was usually after.
    fn nvs_core_os_hostname(_ctx, _args: [0]) {
        let Some(name) = nvs_runtime::os::hostname() else {
            // no case can reach this: a host that will not name itself is not a
            // state a program can produce, so nothing in either suite can drive
            // it. `nvs_runtime::os`'s
            // `the_host_answers_every_fact_this_platform_keeps` asserts the
            // other side of it — that the call does answer, on both platforms.
            return Err(Fault::thrown(
                "`Core\\Os::hostname` has no name to answer with: this host would not report \
                 one, or reported one that is not UTF-8"
                    .to_owned(),
            ));
        };
        Ok(Value::str(NvsStr::new(name.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Os::cpuCount(): uint` — the number `nvs serve` fans its workers
    /// out over, so that a program sizing a pool of its own agrees with the
    /// runtime rather than guessing.
    fn nvs_core_os_cpu_count(_ctx, _args: [0]) {
        let Some(count) = nvs_runtime::os::cpu_count() else {
            // no case can reach this: `available_parallelism` fails only where
            // the operating system refuses to report a mask at all, which no
            // program can bring about. `nvs_runtime::os`'s
            // `the_host_answers_every_fact_this_platform_keeps` asserts it
            // answers instead.
            return Err(Fault::thrown(
                "`Core\\Os::cpuCount` has no count to answer with: this host would not say how \
                 many cores this process may use"
                    .to_owned(),
            ));
        };
        Ok(Value::uint(u64::from(count)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Os::residentBytes(): uint` — the process's resident set, and
    /// deliberately not the request's held bytes, which are `Core\Budget`'s
    /// ([0148](/docs/decisions/0148.md) § 12).
    fn nvs_core_os_resident_bytes(_ctx, _args: [0]) {
        let Some(bytes) = nvs_runtime::os::resident_bytes() else {
            // no case can reach this: a process can always be asked about its
            // own memory, and no program can withdraw that.
            // `nvs_runtime::os`'s
            // `the_host_answers_every_fact_this_platform_keeps` asserts the
            // answer arrives.
            return Err(Fault::thrown(
                "`Core\\Os::residentBytes` has no figure to answer with: this host would not \
                 report what this process holds"
                    .to_owned(),
            ));
        };
        Ok(Value::uint(bytes))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Os::loadAverage(): float[]` — replacing `sys_getloadavg`, and the
    /// one member here with a platform that cannot answer it.
    ///
    /// The refusal names the platform because that is the actionable half: a
    /// program reading this on Windows has not hit a transient failure to retry
    /// but a fact about where it is running.
    fn nvs_core_os_load_average(_ctx, _args: [0]) {
        let Some(average) = nvs_runtime::os::load_average() else {
            // no case can reach this: the throw is Windows's alone, so a case
            // freezing its message would skip on every Unix runner and a case
            // freezing the answer would fail on Windows.
            // `load_average_throws_on_windows_with_a_message_naming_the_platform`
            // asserts it, on whichever platform the run is on.
            return Err(Fault::thrown(format!(
                "`Core\\Os::loadAverage` has no answer on {}: this platform keeps no load \
                 average, and a figure invented here would read as one the operating system \
                 reported",
                std::env::consts::OS
            )));
        };
        let mut answer = NvsArray::new();
        for figure in average {
            answer.append(Value::float(figure));
        }
        Ok(Value::array(answer))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, Value, call};

    use crate::registry::{CoreTy, Qual};

    /// The rendered answer, or the sentence the throw carried — which is what a
    /// `catch` in a program reads, so it is what these assertions compare.
    fn answer(function: nvs_runtime::NvsFn) -> Result<Value, String> {
        let mut ctx = Ctx::buffered();
        match call(function, &mut ctx, &[]) {
            Ok(value) => Ok(value),
            Err(_) => Err(ctx
                .take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()),
        }
    }

    /// Every member of the class answers on the platform the tests run on,
    /// except `loadAverage` on Windows, which has nothing to answer from.
    ///
    /// Asserted as *shapes* and not as values: a pid, a hostname and a resident
    /// set differ on every run, and what could go wrong at this layer is a
    /// member wired to the wrong primitive, which shows up as a zero, an empty
    /// string or a throw rather than as a wrong-looking number.
    // covers: Core\Os::hostname, Core\Os::pid, Core\Os::residentBytes
    #[test]
    fn pid_hostname_cpu_count_resident_bytes_and_load_average_all_answer() {
        assert!(
            answer(super::nvs_core_os_pid)
                .expect("a process has a pid")
                .as_uint()
                .is_some_and(|pid| pid > 0),
            "`pid` answered no process number"
        );
        let hostname = answer(super::nvs_core_os_hostname).expect("a host has a name");
        assert!(
            hostname.as_text().is_some_and(|name| !name.is_empty()),
            "`hostname` answered an empty name"
        );
        assert!(
            answer(super::nvs_core_os_cpu_count)
                .expect("a process runs on at least one core")
                .as_uint()
                .is_some_and(|count| count > 0),
            "`cpuCount` answered no cores"
        );
        assert!(
            answer(super::nvs_core_os_resident_bytes)
                .expect("a running process holds memory")
                .as_uint()
                .is_some_and(|bytes| bytes > 0),
            "`residentBytes` answered nothing held"
        );
        assert_eq!(
            answer(super::nvs_core_os_load_average).is_ok(),
            cfg!(unix),
            "the load average is the one fact Windows keeps none of"
        );
    }

    /// `cpuCount` answers the same number the server fans its workers out over,
    /// which is the whole reason the member is worth having: a program sizing a
    /// pool of its own agrees with the runtime instead of reading the machine's
    /// physical cores and oversubscribing a container.
    // covers: Core\Os::cpuCount
    #[test]
    fn cpu_count_answers_the_number_serve_fans_out_over() {
        let fanout = std::thread::available_parallelism().expect("a host reports its parallelism");
        assert_eq!(
            answer(super::nvs_core_os_cpu_count)
                .expect("a host that reports its parallelism answers this member")
                .as_uint(),
            u64::try_from(fanout.get()).ok(),
            "`cpuCount` and the number `nvs serve` sizes its fleet from have diverged"
        );
    }

    /// The one member with a platform that cannot answer it says so and names
    /// the platform, rather than answering three zeroes a program would act on.
    ///
    /// Both arms are asserted here rather than in a `#[cfg(windows)]` test,
    /// because the claim is a *difference* between the platforms and a test
    /// compiled on one of them states only half of it.
    // covers: Core\Os::loadAverage
    #[test]
    fn load_average_throws_on_windows_with_a_message_naming_the_platform() {
        // Bound rather than written into the assertions, because `assert!(cfg!(…))`
        // is a constant assertion and clippy refuses one.
        let keeps_one = cfg!(unix);
        match answer(super::nvs_core_os_load_average) {
            Ok(value) => {
                assert!(
                    keeps_one,
                    "`loadAverage` answered on a platform that keeps no load average"
                );
                let array = value.array_ptr().expect("`loadAverage` answers an array");
                #[expect(
                    unsafe_code,
                    reason = "the value the call just answered owns a reference to a live \
                              allocation, so it is live for the length of this assertion"
                )]
                let count = unsafe { nvs_runtime::nvs_array_count(array) };
                assert_eq!(count, 3, "a load average is one, five and fifteen minutes");
            }
            Err(message) => {
                assert!(
                    !keeps_one,
                    "`loadAverage` refused on a platform that keeps one: {message}"
                );
                assert!(
                    message.contains(std::env::consts::OS),
                    "the refusal has to name the platform, and this one said: {message}"
                );
            }
        }
    }

    /// Nothing on this class is a sink and nothing it answers is `tainted`.
    ///
    /// These are facts about the process, not about the world: nothing outside
    /// the host wrote a pid or a hostname, and no member's argument becomes an
    /// instruction — there are no arguments at all. The sweep is over the rows
    /// rather than over a list of member names, so a member added later is
    /// covered by it without anyone remembering to come back.
    #[test]
    fn no_core_os_member_is_a_sink_or_carries_tainted() {
        /// `tainted` nests through an array, which is the only shape a `Core`
        /// row has to carry it in.
        fn tainted(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::TaintedStr | CoreTy::TaintedBytes => true,
                CoreTy::Array(inner) => tainted(inner),
                _ => false,
            }
        }

        for member in super::CLASS.methods.iter().chain(super::CLASS.instance) {
            for param in member.params {
                assert_ne!(
                    param.classification(),
                    Some(Qual::Sink),
                    "`{}::{}` declares a sink parameter, and nothing this class is handed \
                     becomes an instruction",
                    super::NAME,
                    member.name
                );
                assert!(
                    !tainted(param),
                    "`{}::{}` takes a `tainted` argument",
                    super::NAME,
                    member.name
                );
            }
            assert!(
                !tainted(&member.return_ty),
                "`{}::{}` answers `tainted`, but nothing outside this host wrote what it reads",
                super::NAME,
                member.name
            );
        }
    }
}
