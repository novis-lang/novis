use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

/// A target's name, the directory its seeds are in, and its check.
type Target = (&'static str, &'static str, fn(&[u8]));

/// Each target with a check, the directory under `fuzz/seeds/` its inputs are
/// in, and the check. `ast` reads `parse`'s seeds, since both take one source
/// text, and `json_reader` reads `json`'s, since both take one JSON text.
const TARGETS: [Target; 18] = [
    ("ast", "parse", crate::ast),
    ("cookie", "cookie", crate::cookie),
    ("decompress", "decompress", crate::decompress),
    ("image_path", "image_path", crate::image_path),
    ("json", "json", crate::json),
    ("json_reader", "json", crate::json_reader),
    ("lex", "lex", crate::lex),
    ("markdown", "markdown", crate::markdown),
    (
        "markdown_from_html",
        "markdown_from_html",
        crate::markdown_from_html,
    ),
    ("multipart", "multipart", crate::multipart),
    ("parse", "parse", crate::parse),
    ("prefix", "prefix", crate::prefix),
    ("regex", "regex", crate::regex),
    ("reply_head", "reply_head", crate::reply_head),
    ("request_head", "request_head", crate::request_head),
    ("serialize", "serialize", crate::serialize),
    ("uri", "uri", crate::uri),
    ("xml", "xml", crate::xml),
];

/// Every file under `fuzz/seeds/<dir>/`, sorted. A directory that does not
/// exist holds no inputs.
fn seeds(dir: &str) -> Vec<PathBuf> {
    let dir = nvs_repo::path(&format!("fuzz/seeds/{dir}"));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut seeds: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a readable seed").path())
        .filter(|path| path.is_file())
        .collect();
    seeds.sort();
    seeds
}

/// The text a caught panic carries.
fn message(cause: &(dyn std::any::Any + Send)) -> &str {
    cause
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| cause.downcast_ref::<&str>().copied())
        .unwrap_or("a panic with no message")
}

#[test]
#[cfg_attr(
    miri,
    ignore = "parses, renders and decodes every saved input, which Miri takes minutes for"
)]
fn every_saved_fuzz_input_passes_its_targets_check() {
    let mut checked = 0usize;
    let mut failed = Vec::new();
    for (target, dir, check) in TARGETS {
        for seed in seeds(dir) {
            let data = std::fs::read(&seed)
                .unwrap_or_else(|why| panic!("{} is readable: {why}", seed.display()));
            checked += 1;
            if let Err(cause) = catch_unwind(AssertUnwindSafe(|| check(&data))) {
                failed.push(format!(
                    "{} fails the `{target}` check: {}",
                    seed.display(),
                    message(&*cause)
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    // `fuzz/seeds/` is committed, so finding none means the path is wrong.
    assert!(checked > 0, "no seed was found under fuzz/seeds/");
}

/// Every saved input of the `compile` target. They stay out of [`TARGETS`],
/// because each runs the whole compiler and the matrix below is this
/// target's other half.
#[test]
#[cfg_attr(miri, ignore = "runs the whole compiler over each input")]
fn compile_seeds_never_panic() {
    let seeds = seeds("compile");
    assert!(
        !seeds.is_empty(),
        "no seed was found under fuzz/seeds/compile/"
    );
    let mut failed = Vec::new();
    for seed in &seeds {
        let data = std::fs::read(seed)
            .unwrap_or_else(|why| panic!("{} is readable: {why}", seed.display()));
        if let Err(cause) = catch_unwind(AssertUnwindSafe(|| crate::compile(&data))) {
            failed.push(format!("{}: {}", seed.display(), message(&*cause)));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// A kind of body a construct can be written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    /// The file's own statements.
    Script,
    Method,
    StaticMethod,
    Generator,
    StaticGenerator,
    /// An anonymous function with a block body, in the body named.
    Block(Host),
    /// An anonymous function with an expression body, in the body named.
    Arrow(Host),
}

/// A body an anonymous function is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Host {
    Script,
    Method,
    StaticMethod,
    Generator,
    StaticGenerator,
}

impl Host {
    const ALL: [Self; 5] = [
        Self::Script,
        Self::Method,
        Self::StaticMethod,
        Self::Generator,
        Self::StaticGenerator,
    ];
}

/// Where in its body a construct is written. Each place but the first starts
/// new blocks, and a value the body defined earlier has to reach them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// The body's own statements.
    Plain,
    /// After a `try` whose thrown error was caught.
    AfterCatch,
    /// In a `catch` block.
    InCatch,
    /// In a `finally` block.
    InFinally,
    /// In a loop.
    InLoop,
}

impl Place {
    const ALL: [Self; 5] = [
        Self::Plain,
        Self::AfterCatch,
        Self::InCatch,
        Self::InFinally,
        Self::InLoop,
    ];

    /// The statements that put `stmt` in this place.
    fn wrap(self, stmt: &str) -> String {
        const THROW: &str = "if (Base::flag()) { throw new LogicError('stop'); }";
        match self {
            Self::Plain => stmt.to_owned(),
            Self::AfterCatch => {
                format!("try {{ {THROW} }} catch (LogicError $e) {{ echo 0; }}\n{stmt}")
            }
            Self::InCatch => format!("try {{ {THROW} }} catch (LogicError $e) {{\n{stmt}\n}}"),
            Self::InFinally => format!("try {{ echo 0; }} finally {{\n{stmt}\n}}"),
            Self::InLoop => format!("int $i = 0;\nwhile ($i < 2) {{\n$i = $i + 1;\n{stmt}\n}}"),
        }
    }
}

/// A construct that reads the body around it: its statement form, and its
/// expression form where it has one. `{ret}` is what a `return` gives in the
/// body it lands in.
struct Construct {
    name: &'static str,
    stmt: &'static str,
    expr: Option<&'static str>,
}

const CONSTRUCTS: &[Construct] = &[
    Construct {
        name: "self call",
        stmt: "echo self::one();",
        expr: Some("self::one()"),
    },
    Construct {
        name: "static call",
        stmt: "echo static::one();",
        expr: Some("static::one()"),
    },
    Construct {
        name: "parent call",
        stmt: "echo parent::one();",
        expr: Some("parent::one()"),
    },
    Construct {
        name: "self constant",
        stmt: "echo self::K;",
        expr: Some("self::K"),
    },
    Construct {
        name: "static constant",
        stmt: "echo static::K;",
        expr: Some("static::K"),
    },
    Construct {
        name: "parent constant",
        stmt: "echo parent::K;",
        expr: Some("parent::K"),
    },
    Construct {
        name: "$this",
        stmt: "echo $this->two();",
        expr: Some("$this->two()"),
    },
    Construct {
        name: "new static",
        stmt: "Base $o = new static(); echo $o->two();",
        expr: Some("(new static())->two()"),
    },
    Construct {
        name: "captured variable",
        stmt: "echo $c;",
        expr: Some("$c"),
    },
    Construct {
        name: "yield",
        stmt: "yield 7;",
        expr: None,
    },
    Construct {
        name: "return",
        stmt: "if (Base::flag()) { return{ret}; }",
        expr: None,
    },
    Construct {
        name: "throw",
        stmt: "if (Base::flag()) { throw new LogicError('stop'); }",
        expr: None,
    },
    Construct {
        name: "try",
        stmt: "try { echo self::one(); } catch (LogicError $e) { echo static::K; } \
               finally { echo parent::K; }",
        expr: None,
    },
    Construct {
        name: "yield in try",
        stmt: "try { yield 1; } finally { echo self::K; }",
        expr: None,
    },
    Construct {
        name: "match",
        stmt: "echo match (static::K) { 1 => 'one', default => 'other' };",
        expr: Some("match (static::K) { 1 => 3, default => 4 }"),
    },
];

/// The program that puts `c` into `body` at `place`, or `None` where the
/// body has no such place: an expression body takes only an expression.
fn cell(body: Body, place: Place, c: &Construct) -> Option<String> {
    if matches!(body, Body::Arrow(_)) && place != Place::Plain {
        return None;
    }
    let stmt = place.wrap(c.stmt);
    let stmt = stmt.as_str();
    // What a body returns, as its own statements and the `return` value.
    fn wrap(host: Host, inner: &str) -> (String, String) {
        let ret = match host {
            Host::Method | Host::StaticMethod => " 1",
            _ => "",
        };
        let stmts = format!("int $c = 4;\n{}", inner.replace("{ret}", ret));
        match host {
            Host::Script => (stmts, String::new()),
            Host::Method | Host::StaticMethod | Host::Generator | Host::StaticGenerator => {
                let modifier = match host {
                    Host::StaticMethod | Host::StaticGenerator => "static ",
                    _ => "",
                };
                let (ty, tail) = match host {
                    Host::Generator | Host::StaticGenerator => ("Iterator<int>", "yield 0;"),
                    _ => ("int", "return 0;"),
                };
                (
                    String::new(),
                    format!("public {modifier}function body(): {ty}\n{{\n{stmts}\n{tail}\n}}"),
                )
            }
        }
    }
    let (script, member) = match body {
        Body::Script => wrap(Host::Script, stmt),
        Body::Method => wrap(Host::Method, stmt),
        Body::StaticMethod => wrap(Host::StaticMethod, stmt),
        Body::Generator => wrap(Host::Generator, stmt),
        Body::StaticGenerator => wrap(Host::StaticGenerator, stmt),
        Body::Block(host) => {
            let inner = stmt.replace("{ret}", " 1");
            let f = format!(
                "callable(): int $f = fn(): int => {{\n{inner}\nreturn 0;\n}};\necho $f();"
            );
            wrap(host, &f)
        }
        Body::Arrow(host) => {
            let f = format!("callable(): int $f = fn(): int => {};\necho $f();", c.expr?);
            wrap(host, &f)
        }
    };
    Some(format!(
        "<?nvs\n\
         class Base\n{{\n\
         public const int K = 1;\n\
         public static function one(): int {{ return 1; }}\n\
         public function two(): int {{ return 2; }}\n\
         public static function flag(): bool {{ return true; }}\n\
         }}\n\
         class Probe extends Base\n{{\n\
         public const int K = 2;\n\
         public static function one(): int {{ return 10; }}\n\
         {member}\n\
         }}\n\
         {script}\n"
    ))
}

/// Each construct that reads the body around it, in each kind of body and at
/// each place in it, through the whole compiler. A cell may fail to
/// type-check, since `$this` has no meaning in a static method, but no cell
/// panics, and a cell that checks is built. The cells run in parallel, one
/// thread per core.
#[test]
#[cfg_attr(miri, ignore = "runs the whole compiler over each cell")]
fn every_construct_compiles_in_every_body() {
    let mut bodies = vec![
        Body::Script,
        Body::Method,
        Body::StaticMethod,
        Body::Generator,
        Body::StaticGenerator,
    ];
    for host in Host::ALL {
        bodies.push(Body::Block(host));
        bodies.push(Body::Arrow(host));
    }
    let cells: Vec<(Body, Place, &Construct, String)> = bodies
        .iter()
        .flat_map(|&body| Place::ALL.into_iter().map(move |place| (body, place)))
        .flat_map(|(body, place)| {
            CONSTRUCTS
                .iter()
                .filter_map(move |c| cell(body, place, c).map(|program| (body, place, c, program)))
        })
        .collect();

    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk = cells.len().div_ceil(threads);
    type Outcome<'c> = (Body, Place, &'c str, Result<crate::Compiled, String>);
    let outcomes: Vec<Outcome<'_>> = std::thread::scope(|scope| {
        let handles: Vec<_> = cells
            .chunks(chunk)
            .map(|part| {
                std::thread::Builder::new()
                    .stack_size(32 << 20)
                    .spawn_scoped(scope, move || {
                        part.iter()
                            .map(|(body, place, c, program)| {
                                let outcome = catch_unwind(AssertUnwindSafe(|| {
                                    crate::compile_source(program)
                                }));
                                let outcome = match outcome {
                                    Ok(Ok(compiled)) => Ok(compiled),
                                    Ok(Err(error)) => Err(format!("code generation: {error}")),
                                    Err(cause) => Err(format!("panic: {}", message(&*cause))),
                                };
                                (*body, *place, c.name, outcome)
                            })
                            .collect::<Vec<_>>()
                    })
                    .expect("a matrix thread starts")
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a matrix thread ends"))
            .collect()
    });

    let failed: Vec<String> = outcomes
        .iter()
        .filter_map(|(body, place, name, outcome)| {
            outcome
                .as_ref()
                .err()
                .map(|why| format!("{name} in {body:?}, {place:?}: {why}"))
        })
        .collect();
    assert!(
        failed.is_empty(),
        "{} of {} cells failed:\n{}",
        failed.len(),
        outcomes.len(),
        failed.join("\n")
    );

    // The cells a correct compiler builds. Each row below type-checks, so a
    // scaffold that stops compiling shows here, and not as a matrix that
    // checks nothing.
    let built = |body: Body, place: Place, name: &str| {
        outcomes.iter().any(|(b, p, n, o)| {
            *b == body && *p == place && *n == name && *o == Ok(crate::Compiled::Built)
        })
    };
    for (body, place, name) in [
        (Body::StaticGenerator, Place::Plain, "self call"),
        (Body::StaticGenerator, Place::Plain, "static call"),
        (Body::Generator, Place::Plain, "self call"),
        (Body::Generator, Place::Plain, "new static"),
        (Body::Method, Place::Plain, "try"),
        (Body::Method, Place::AfterCatch, "static constant"),
        (Body::Arrow(Host::Method), Place::Plain, "$this"),
    ] {
        assert!(
            built(body, place, name),
            "`{name}` in {body:?}, {place:?}, is expected to build"
        );
    }
}
