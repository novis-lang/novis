//! The intl component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/intl-batch-shape`): `Novis\Intl\Icu` is in the binary and answers with no
//! `[[extension]]` entry, a sort of 10,000 strings is one crossing, its sort keys order as its
//! collator does, and the order is the locale's. A batch of numbers is formatted in one crossing in
//! all four styles, and its plural categories agree with `Core\Cldr`'s on that member's whole
//! roster. A date and time is formatted from the local fields and the offset the host sent, in its
//! locale, and the guest carries no time-zone database. A batch of strings is cut into words and
//! sentences in one crossing, and an `Accept-Language` value picks an offered locale by weight and
//! CLDR fallback, or the default, and never throws for the header.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{INTL, INTL_CLASS, INTL_SHA256};
use nvs_ext::call::{Error, Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Builtin, Extension, Loader};

/// The intl component among `builtins`.
fn intl(builtins: &[Builtin]) -> &Builtin {
    builtins
        .iter()
        .find(|builtin| builtin.manifest().class == INTL_CLASS)
        .expect("the intl component is built in")
}

/// `future` polled on this thread until it finishes. A guest call yields at every epoch tick, and
/// polling again resumes it.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

fn list(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(
        items
            .into_iter()
            .enumerate()
            .map(|(index, item)| (Key::Int(i64::try_from(index).unwrap()), item))
            .collect(),
    )
}

fn strings(items: &[String]) -> Value {
    list(items.iter().cloned().map(Value::String))
}

fn items(value: Value) -> Vec<Value> {
    let Value::Array(entries) = value else {
        panic!("a list was expected, not {value:?}");
    };
    entries.into_iter().map(|(_, item)| item).collect()
}

/// `method` called with `strings`, `locale` and no options.
fn collate(
    request: &Request,
    extension: &Extension,
    method: &str,
    input: &[String],
    locale: &str,
) -> Result<Vec<Value>, Failure> {
    let args = vec![
        strings(input),
        Value::String(locale.to_owned()),
        Value::Array(Vec::new()),
    ];
    let result = block_on(request.call_values(extension, method, args))?;
    Ok(items(result.expect("the method returns a value")))
}

/// The order `collateOrder` returns, as indexes.
fn order(request: &Request, extension: &Extension, input: &[String], locale: &str) -> Vec<usize> {
    collate(request, extension, "collateOrder", input, locale)
        .expect("`collateOrder` answers")
        .into_iter()
        .map(|index| match index {
            Value::Uint(index) => usize::try_from(index).unwrap(),
            other => panic!("an index was expected, not {other:?}"),
        })
        .collect()
}

fn keys(request: &Request, extension: &Extension, input: &[String], locale: &str) -> Vec<Vec<u8>> {
    collate(request, extension, "sortKeys", input, locale)
        .expect("`sortKeys` answers")
        .into_iter()
        .map(|key| match key {
            Value::Bytes(key) => key,
            other => panic!("a key was expected, not {other:?}"),
        })
        .collect()
}

/// `count` names from a fixed seed, with accented letters and both cases, so ties and tailorings
/// are both in the batch.
fn names(count: usize) -> Vec<String> {
    const LETTERS: [&str; 16] = [
        "a", "b", "e", "k", "o", "s", "z", "A", "Z", "ä", "ö", "é", "Å", "ß", "-", " ",
    ];
    let nibble = |state: u64, i: usize| usize::try_from((state >> (4 * i)) & 0xF).unwrap();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let len = 1 + nibble(state, 15) % 8;
            (0..len).map(|i| LETTERS[nibble(state, i)]).collect()
        })
        .collect()
}

fn run<T>(test: impl FnOnce(&Request, &Extension) -> T) -> T {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = intl(&builtins)
        .extension()
        .expect("the intl component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let out = test(&request, extension);
    block_on(request.end()).expect("the request ends");
    out
}

#[test]
fn the_intl_component_is_built_in_and_answers_with_no_configuration() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let builtin = intl(&builtins);
    let digest: String = INTL_SHA256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(builtin.sha256(), digest);
    assert_eq!(builtin.sha256(), nvs_ext::load::pin(INTL));
    assert!(!builtin.compiled(), "loading compiled the component");
    let extension = builtin
        .extension()
        .expect("the intl component compiles and checks");
    assert_eq!(extension.memory, None);
    assert_eq!(extension.grants, nvs_config::extension::Granted::default());
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let input = ["b".to_owned(), "a".to_owned()];
    assert_eq!(order(&request, extension, &input, "en"), [1, 0]);
    match collate(&request, extension, "collateOrder", &input, "en_US!") {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(message.contains("en_US!"), "{message}");
        }
        other => panic!("a malformed tag was not `invalid`: {other:?}"),
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn sorting_ten_thousand_strings_is_one_crossing() {
    let input = names(10_000);
    let (sorted, crossings) = run(|request, extension| {
        let before = request.crossings();
        let sorted = order(request, extension, &input, "de");
        (sorted, request.crossings() - before)
    });
    assert_eq!(crossings, 1);
    let mut seen = sorted.clone();
    seen.sort_unstable();
    assert_eq!(
        seen,
        (0..input.len()).collect::<Vec<_>>(),
        "not a permutation"
    );
}

// covers: Novis\Intl\Collator::sortKeys, Novis\Intl\Icu::sortKeys, Novis\Intl\Icu::collateOrder
#[test]
fn sort_keys_order_as_the_collator_orders() {
    let input = names(2_000);
    let (sorted, keys) = run(|request, extension| {
        (
            order(request, extension, &input, "de"),
            keys(request, extension, &input, "de"),
        )
    });
    assert_eq!(keys.len(), input.len());
    // Both sorts are stable, so equal strings are in input order on both sides.
    let mut by_key: Vec<usize> = (0..input.len()).collect();
    by_key.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
    assert_eq!(by_key, sorted);
}

// covers: Novis\Intl\Collator::sort
#[test]
fn a_swedish_sort_puts_a_umlaut_after_z_and_a_german_sort_does_not() {
    let input: Vec<String> = ["ä", "z", "a"].iter().map(|s| (*s).to_owned()).collect();
    let (swedish, german) = run(|request, extension| {
        (
            order(request, extension, &input, "sv"),
            order(request, extension, &input, "de"),
        )
    });
    let words = |order: Vec<usize>| -> Vec<&str> { order.iter().map(|&i| &*input[i]).collect() };
    assert_eq!(words(swedish), ["a", "z", "ä"]);
    assert_eq!(words(german), ["a", "ä", "z"]);
}

fn texts(numbers: &[&str]) -> Vec<String> {
    numbers.iter().map(|n| (*n).to_owned()).collect()
}

/// The `number-options` record for `style`, with `fields` set and every other field `null`.
fn number_options(style: &str, fields: &[(&str, Value)]) -> Value {
    let names = [
        "minFractionDigits",
        "maxFractionDigits",
        "grouping",
        "currency",
        "currencyDisplay",
        "compactDisplay",
    ];
    let mut record = vec![(
        Key::String("style".to_owned()),
        Value::Case(style.to_owned()),
    )];
    for name in names {
        let value = fields
            .iter()
            .find(|(field, _)| *field == name)
            .map_or(Value::Null, |(_, value)| value.clone());
        record.push((Key::String(name.to_owned()), value));
    }
    Value::Array(record)
}

/// `formatNumbers` over `numbers` in `locale` under `options`.
fn format(
    request: &Request,
    extension: &Extension,
    numbers: &[String],
    locale: &str,
    options: Value,
) -> Vec<String> {
    let args = vec![strings(numbers), Value::String(locale.to_owned()), options];
    let result = block_on(request.call_values(extension, "formatNumbers", args))
        .expect("`formatNumbers` answers");
    items(result.expect("the method returns a value"))
        .into_iter()
        .map(|text| match text {
            Value::String(text) => text,
            other => panic!("a string was expected, not {other:?}"),
        })
        .collect()
}

/// `pluralCategories` over `numbers` in `locale`, each category by its case name.
fn plurals(
    request: &Request,
    extension: &Extension,
    numbers: &[String],
    locale: &str,
    kind: &str,
) -> Vec<String> {
    let args = vec![
        strings(numbers),
        Value::String(locale.to_owned()),
        Value::Case(kind.to_owned()),
    ];
    let result = block_on(request.call_values(extension, "pluralCategories", args))
        .expect("`pluralCategories` answers");
    items(result.expect("the method returns a value"))
        .into_iter()
        .map(|category| match category {
            Value::Case(name) => name,
            other => panic!("a category was expected, not {other:?}"),
        })
        .collect()
}

// covers: Novis\Intl\NumberFormat::decimal, Novis\Intl\NumberFormat::percent, Novis\Intl\NumberFormat::currency, Novis\Intl\NumberFormat::compact, Novis\Intl\Icu::formatNumbers
#[test]
fn decimal_percent_currency_and_compact_format_by_locale() {
    run(|request, extension| {
        let numbers = texts(&["1234567.891", "-0.0001", "19.90"]);
        assert_eq!(
            format(
                request,
                extension,
                &numbers,
                "en",
                number_options("Decimal", &[])
            ),
            ["1,234,567.891", "0", "19.9"]
        );
        assert_eq!(
            format(
                request,
                extension,
                &numbers,
                "de",
                number_options("Decimal", &[])
            ),
            ["1.234.567,891", "0", "19,9"]
        );
        let two_digits = number_options(
            "Decimal",
            &[
                ("minFractionDigits", Value::Uint(2)),
                ("maxFractionDigits", Value::Uint(2)),
                ("grouping", Value::Bool(false)),
            ],
        );
        assert_eq!(
            format(
                request,
                extension,
                &texts(&["1234.5", "1.0E+3"]),
                "en",
                two_digits
            ),
            ["1234.50", "1000.00"]
        );
        assert_eq!(
            format(
                request,
                extension,
                &texts(&["0.25", "-0.125"]),
                "en",
                number_options("Percent", &[])
            ),
            ["25%", "-13%"]
        );
        let euro = |display: &str| {
            number_options(
                "Currency",
                &[
                    ("currency", Value::String("EUR".to_owned())),
                    ("currencyDisplay", Value::Case(display.to_owned())),
                ],
            )
        };
        assert_eq!(
            format(
                request,
                extension,
                &texts(&["1234.5"]),
                "de",
                euro("Symbol")
            ),
            ["1.234,50\u{a0}€"]
        );
        assert_eq!(
            format(request, extension, &texts(&["2"]), "en", euro("Name")),
            ["2.00 euros"]
        );
        assert_eq!(
            format(
                request,
                extension,
                &texts(&["1234", "1500000"]),
                "en",
                number_options("Compact", &[])
            ),
            ["1.2K", "1.5M"]
        );
        let long = number_options(
            "Compact",
            &[("compactDisplay", Value::Case("Long".to_owned()))],
        );
        assert_eq!(
            format(request, extension, &texts(&["1234"]), "de", long),
            ["1,2 Tausend"]
        );
        // An option the style does not take is `invalid`, and so is a currency code that is not one.
        for (options, what) in [
            (
                number_options("Compact", &[("grouping", Value::Bool(true))]),
                "grouping",
            ),
            (
                number_options(
                    "Currency",
                    &[("currency", Value::String("EURO".to_owned()))],
                ),
                "EURO",
            ),
        ] {
            let args = vec![
                strings(&texts(&["1"])),
                Value::String("en".to_owned()),
                options,
            ];
            match block_on(request.call_values(extension, "formatNumbers", args)) {
                Err(Failure::Error(Error::Invalid(message))) => {
                    assert!(message.contains(what), "{message}");
                }
                other => panic!("`{what}` was not `invalid`: {other:?}"),
            }
        }
    });
}

#[test]
fn a_batch_of_numbers_formats_in_one_crossing() {
    let numbers: Vec<String> = (0..5_000)
        .map(|i| format!("{}.{:02}", i * 37, i % 100))
        .collect();
    let (formatted, crossings) = run(|request, extension| {
        let before = request.crossings();
        let formatted = format(
            request,
            extension,
            &numbers,
            "en",
            number_options("Decimal", &[]),
        );
        (formatted, request.crossings() - before)
    });
    assert_eq!(crossings, 1);
    assert_eq!(formatted.len(), numbers.len());
    assert_eq!(formatted[100], "3,700");
    assert_eq!(formatted[4_999], "184,963.99");
}

// covers: Novis\Intl\PluralRules::cardinal, Novis\Intl\PluralRules::ordinal, Novis\Intl\Icu::pluralCategories
#[test]
fn plural_and_ordinal_categories_of_a_batch() {
    let (english, russian, ordinals, crossings) = run(|request, extension| {
        let before = request.crossings();
        let english = plurals(
            request,
            extension,
            &texts(&["1", "1.0", "2", "0"]),
            "en",
            "Cardinal",
        );
        let russian = plurals(
            request,
            extension,
            &texts(&["1", "2", "5", "21", "1.5"]),
            "ru",
            "Cardinal",
        );
        let ordinals = plurals(
            request,
            extension,
            &texts(&["1", "2", "3", "4", "11", "22"]),
            "en",
            "Ordinal",
        );
        (english, russian, ordinals, request.crossings() - before)
    });
    assert_eq!(crossings, 3, "one crossing per batch");
    assert_eq!(english, ["One", "Other", "Other", "Other"]);
    assert_eq!(russian, ["One", "Few", "Many", "One", "Other"]);
    assert_eq!(ordinals, ["One", "Two", "Few", "Other", "Other", "Two"]);
}

#[test]
fn plural_rules_agree_with_core_cldr_on_its_whole_roster() {
    let mut counts: Vec<String> = (0..=120).map(|n: u32| n.to_string()).collect();
    for n in ["1000", "10000", "100000", "1000000", "1000001", "2000000"] {
        counts.push(n.to_owned());
    }
    for n in [
        "0.0", "0.1", "0.5", "1.0", "1.5", "2.0", "2.5", "3.4", "5.0", "10.1", "11.0", "21.0",
        "1.25", "0.01", "100.25",
    ] {
        counts.push(n.to_owned());
    }
    let languages: Vec<&str> = nvs_stdlib::cldr::plural_languages().collect();
    assert!(
        languages.len() > 50,
        "the roster is {} languages",
        languages.len()
    );
    // One line per language and kind that disagrees, with its first three counts.
    let disagreements = run(|request, extension| {
        let mut disagreements = Vec::new();
        for &language in &languages {
            for (kind, ordinal) in [("Cardinal", false), ("Ordinal", true)] {
                let intl = plurals(request, extension, &counts, language, kind);
                let differ: Vec<String> = counts
                    .iter()
                    .zip(intl)
                    .filter_map(|(count, intl)| {
                        let cldr = nvs_stdlib::cldr::plural_category_name(count, language, ordinal)
                            .expect("`Core\\Cldr` carries every language on its roster");
                        (intl != cldr).then(|| format!("{count}: {intl} != {cldr}"))
                    })
                    .collect();
                if !differ.is_empty() {
                    disagreements.push(format!(
                        "{language} {kind}, {} count(s): {}",
                        differ.len(),
                        differ[..differ.len().min(3)].join(", ")
                    ));
                }
            }
        }
        disagreements
    });
    assert!(
        disagreements.is_empty(),
        "`Novis\\Intl` and `Core\\Cldr` disagree:\n{}",
        disagreements.join("\n")
    );
}

/// The `local-date-time` record of a `Core\Time\DateTime` the host placed in `zone` at
/// `offset_seconds`, at 15:05:09 on `day` July 2026.
fn local(day: i64, offset_seconds: i64, zone: &str) -> Value {
    let fields = [
        ("year", 2026),
        ("month", 7),
        ("day", day),
        ("hour", 15),
        ("minute", 5),
        ("second", 9),
        ("nanos", 0),
        ("offsetSeconds", offset_seconds),
    ];
    let mut record: Vec<(Key, Value)> = fields
        .into_iter()
        .map(|(name, value)| (Key::String(name.to_owned()), Value::Int(value)))
        .collect();
    record.push((
        Key::String("zone".to_owned()),
        Value::String(zone.to_owned()),
    ));
    Value::Array(record)
}

/// An options shape with `fields` set and every other field absent.
fn shape(fields: &[(&str, Value)]) -> Value {
    Value::Array(
        fields
            .iter()
            .map(|(name, value)| (Key::String((*name).to_owned()), value.clone()))
            .collect(),
    )
}

/// `method` called with `values`, `locale` and `last`, its strings in order.
fn written(
    request: &Request,
    extension: &Extension,
    method: &str,
    values: Vec<Value>,
    locale: &str,
    last: Value,
) -> Vec<String> {
    let args = vec![list(values), Value::String(locale.to_owned()), last];
    let result = block_on(request.call_values(extension, method, args))
        .unwrap_or_else(|failure| panic!("`{method}` answers: {failure:?}"));
    items(result.expect("the method returns a value"))
        .into_iter()
        .map(|text| match text {
            Value::String(text) => text,
            other => panic!("a string was expected, not {other:?}"),
        })
        .collect()
}

fn core(class: &str, fields: &[(&str, i64)]) -> Value {
    Value::Core {
        class: class.to_owned(),
        fields: fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), Value::Int(*value)))
            .collect(),
    }
}

// covers: Novis\Intl\DateFormat::dateTimes, Novis\Intl\DateFormat::dates, Novis\Intl\DateFormat::times, Novis\Intl\Icu::formatDateTimes, Novis\Intl\Icu::formatDates, Novis\Intl\Icu::formatTimes
#[test]
fn a_core_time_value_formats_in_its_zone_and_locale() {
    let (date_times, zoned, dates, times, crossings) = run(|request, extension| {
        let before = request.crossings();
        let values = || {
            vec![
                local(14, 7200, "Europe/Vienna"),
                local(15, -14400, "America/New_York"),
            ]
        };
        let date_times = [
            written(
                request,
                extension,
                "formatDateTimes",
                values(),
                "en",
                shape(&[]),
            ),
            written(
                request,
                extension,
                "formatDateTimes",
                values(),
                "de",
                shape(&[]),
            ),
        ];
        let zoned = [
            ("Offset", "en"),
            ("Location", "en"),
            ("Generic", "en"),
            ("Generic", "de"),
        ]
        .map(|(style, locale)| {
            let options = shape(&[
                ("zone", Value::Case(style.to_owned())),
                ("seconds", Value::Bool(true)),
            ]);
            written(
                request,
                extension,
                "formatDateTimes",
                values(),
                locale,
                options,
            )
        });
        let date = core(
            "Core\\Time\\Date",
            &[("year", 2026), ("month", 3), ("day", 1)],
        );
        let dates = ["Short", "Medium", "Long"].map(|length| {
            let length = Value::Case(length.to_owned());
            written(
                request,
                extension,
                "formatDates",
                vec![date.clone()],
                "en-US",
                length,
            )
        });
        let time = core(
            "Core\\Time\\TimeOfDay",
            &[("hour", 21), ("minute", 30), ("second", 15), ("nanos", 0)],
        );
        let times = [("en-US", false), ("de", true), ("ja", false)].map(|(locale, seconds)| {
            let options = shape(&[("seconds", Value::Bool(seconds))]);
            written(
                request,
                extension,
                "formatTimes",
                vec![time.clone()],
                locale,
                options,
            )
        });
        (
            date_times,
            zoned,
            dates,
            times,
            request.crossings() - before,
        )
    });
    assert_eq!(crossings, 12, "one crossing per batch");
    assert_eq!(
        date_times,
        [
            [
                "Jul 14, 2026, 3:05\u{202f}PM",
                "Jul 15, 2026, 3:05\u{202f}PM"
            ],
            ["14.07.2026, 15:05", "15.07.2026, 15:05"],
        ]
    );
    assert_eq!(
        zoned,
        [
            [
                "Jul 14, 2026, 3:05:09\u{202f}PM GMT+2",
                "Jul 15, 2026, 3:05:09\u{202f}PM GMT-4"
            ],
            [
                "Jul 14, 2026, 3:05:09\u{202f}PM Austria Time",
                "Jul 15, 2026, 3:05:09\u{202f}PM New York Time"
            ],
            [
                "Jul 14, 2026, 3:05:09\u{202f}PM Central European Time",
                "Jul 15, 2026, 3:05:09\u{202f}PM Eastern Time"
            ],
            [
                "14.07.2026, 15:05:09 Mitteleuropäische Zeit",
                "15.07.2026, 15:05:09 Nordamerikanische Ostküstenzeit"
            ],
        ]
    );
    assert_eq!(dates, [["3/1/26"], ["Mar 1, 2026"], ["March 1, 2026"]]);
    assert_eq!(times, [["9:30\u{202f}PM"], ["21:30:15"], ["21:30"]]);
}

#[test]
fn the_guest_carries_no_time_zone_database() {
    // The guest writes the offset the host sent. A guest that looked the zone up would write
    // `GMT+2` for Vienna in July whatever the host said.
    let offsets = run(|request, extension| {
        let options = shape(&[("zone", Value::Case("Offset".to_owned()))]);
        written(
            request,
            extension,
            "formatDateTimes",
            vec![
                local(14, 7200, "Europe/Vienna"),
                local(14, 3600, "Europe/Vienna"),
                local(14, 19800, "Europe/Vienna"),
            ],
            "en",
            options,
        )
    });
    assert_eq!(
        offsets,
        [
            "Jul 14, 2026, 3:05\u{202f}PM GMT+2",
            "Jul 14, 2026, 3:05\u{202f}PM GMT+1",
            "Jul 14, 2026, 3:05\u{202f}PM GMT+5:30",
        ]
    );
    // A compiled tz database is a run of TZif files, each starting with this magic.
    assert!(
        !INTL.windows(4).any(|bytes| bytes == b"TZif"),
        "the intl component carries a TZif file"
    );
}

/// The `relative-item` record of `count` `unit`s.
fn relative(count: i64, unit: &str) -> Value {
    shape(&[
        ("count", Value::Int(count)),
        ("unit", Value::Case(unit.to_owned())),
    ])
}

// covers: Novis\Intl\RelativeTime::format, Novis\Intl\Icu::formatRelative
#[test]
fn relative_time_formats_past_and_future() {
    let (written_out, crossings) = run(|request, extension| {
        let before = request.crossings();
        let items = || {
            vec![
                relative(-3, "Day"),
                relative(2, "Hour"),
                relative(-1, "Day"),
                relative(1, "Year"),
            ]
        };
        let written_out = [
            ("en", "Always", "Wide"),
            ("en", "Auto", "Wide"),
            ("de", "Auto", "Wide"),
            ("en", "Always", "Short"),
        ]
        .map(|(locale, numeric, width)| {
            let options = shape(&[
                ("numeric", Value::Case(numeric.to_owned())),
                ("width", Value::Case(width.to_owned())),
            ]);
            written(
                request,
                extension,
                "formatRelative",
                items(),
                locale,
                options,
            )
        });
        (written_out, request.crossings() - before)
    });
    assert_eq!(crossings, 4, "one crossing per batch");
    assert_eq!(
        written_out,
        [
            ["3 days ago", "in 2 hours", "1 day ago", "in 1 year"],
            ["3 days ago", "in 2 hours", "yesterday", "next year"],
            ["vor 3 Tagen", "in 2 Stunden", "gestern", "nächstes Jahr"],
            ["3 days ago", "in 2 hr.", "1 day ago", "in 1 yr."],
        ]
    );
}

// covers: Novis\Intl\ListFormat::join, Novis\Intl\Icu::formatLists
#[test]
fn a_list_formats_with_its_locale_conjunction() {
    let (joined, crossings) = run(|request, extension| {
        let before = request.crossings();
        let lists = || {
            ["Shop", "Blog", "Wiki"]
                .map(|item| Value::String(item.to_owned()))
                .to_vec()
        };
        let joined = [
            ("en", "And"),
            ("de", "And"),
            ("es", "And"),
            ("en", "Or"),
            ("fr", "Or"),
        ]
        .map(|(locale, list_type)| {
            let options = shape(&[("type", Value::Case(list_type.to_owned()))]);
            written(
                request,
                extension,
                "formatLists",
                vec![list(lists())],
                locale,
                options,
            )
        });
        (joined, request.crossings() - before)
    });
    assert_eq!(crossings, 5, "one crossing per batch");
    assert_eq!(
        joined,
        [
            ["Shop, Blog, and Wiki"],
            ["Shop, Blog und Wiki"],
            ["Shop, Blog y Wiki"],
            ["Shop, Blog, or Wiki"],
            ["Shop, Blog ou Wiki"],
        ]
    );
}

/// `method` called with `input` and `locale`, each string's segments in order.
fn segments(
    request: &Request,
    extension: &Extension,
    method: &str,
    input: &[&str],
    locale: &str,
) -> Vec<Vec<Value>> {
    let input: Vec<String> = input.iter().map(|text| (*text).to_owned()).collect();
    let args = vec![strings(&input), Value::String(locale.to_owned())];
    let result = block_on(request.call_values(extension, method, args))
        .unwrap_or_else(|failure| panic!("`{method}` answers: {failure:?}"));
    items(result.expect("the method returns a value"))
        .into_iter()
        .map(items)
        .collect()
}

// covers: Novis\Intl\Segmenter::words, Novis\Intl\Segmenter::sentences, Novis\Intl\Icu::wordSegments, Novis\Intl\Icu::sentenceSegments
#[test]
fn word_and_sentence_segments_of_a_batch_of_strings() {
    let (words, sentences, crossings) = run(|request, extension| {
        let before = request.crossings();
        let input = ["The Shop opens at 9.", "こんにちは世界", ""];
        let words = segments(request, extension, "wordSegments", &input, "ja");
        let sentences = segments(
            request,
            extension,
            "sentenceSegments",
            &["The Shop is open. Is the Blog? Yes!", ""],
            "en",
        );
        (words, sentences, request.crossings() - before)
    });
    assert_eq!(crossings, 2, "one crossing per batch");
    let words: Vec<Vec<(String, bool)>> = words
        .into_iter()
        .map(|string| {
            string
                .into_iter()
                .map(|word| {
                    let Value::Array(fields) = word else {
                        panic!("a word record was expected, not {word:?}");
                    };
                    match fields.as_slice() {
                        [
                            (Key::String(text_key), Value::String(text)),
                            (Key::String(like_key), Value::Bool(like)),
                        ] if text_key == "text" && like_key == "wordLike" => (text.clone(), *like),
                        other => panic!("`{{text, wordLike}}` was expected, not {other:?}"),
                    }
                })
                .collect()
        })
        .collect();
    let expected = |pairs: &[(&str, bool)]| {
        pairs
            .iter()
            .map(|(text, like)| ((*text).to_owned(), *like))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        words,
        [
            expected(&[
                ("The", true),
                (" ", false),
                ("Shop", true),
                (" ", false),
                ("opens", true),
                (" ", false),
                ("at", true),
                (" ", false),
                ("9", true),
                (".", false),
            ]),
            expected(&[("こんにちは", true), ("世界", true)]),
            Vec::new(),
        ]
    );
    let sentences: Vec<Vec<String>> = sentences
        .into_iter()
        .map(|string| {
            string
                .into_iter()
                .map(|sentence| match sentence {
                    Value::String(sentence) => sentence,
                    other => panic!("a string was expected, not {other:?}"),
                })
                .collect()
        })
        .collect();
    assert_eq!(
        sentences,
        [
            vec!["The Shop is open. ", "Is the Blog? ", "Yes!"],
            Vec::new(),
        ]
    );
}

/// `negotiate` called with `header`, `offered` and `default`.
fn negotiated(
    request: &Request,
    extension: &Extension,
    header: &str,
    offered: &[&str],
    default: &str,
) -> Result<String, Failure> {
    let offered: Vec<String> = offered.iter().map(|tag| (*tag).to_owned()).collect();
    let args = vec![
        Value::String(header.to_owned()),
        strings(&offered),
        Value::String(default.to_owned()),
    ];
    match block_on(request.call_values(extension, "negotiate", args))? {
        Some(Value::String(tag)) => Ok(tag),
        other => panic!("a tag was expected, not {other:?}"),
    }
}

// covers: Novis\Intl\Locale::negotiate, Novis\Intl\Icu::negotiate
#[test]
fn accept_language_negotiates_the_best_offered_locale() {
    let (chosen, crossings) = run(|request, extension| {
        let before = request.crossings();
        let offered = ["en", "de", "fr"];
        let chosen = [
            "fr;q=0.5, de, en;q=0.8",
            "ja, fr;q=0.4, de;q=0.4",
            "FR-ca;q=0.9, en;q=0.1",
        ]
        .map(|header| negotiated(request, extension, header, &offered, "en").unwrap());
        (chosen, request.crossings() - before)
    });
    assert_eq!(crossings, 3, "one crossing per header");
    assert_eq!(chosen, ["de", "fr", "fr"]);
}

// covers: Novis\Intl\Locale::resolve, Novis\Intl\Icu::resolveLocales
#[test]
fn a_regional_range_falls_back_to_its_language() {
    let (chosen, resolved) = run(|request, extension| {
        let offered = ["en", "de", "pt-BR"];
        let chosen = ["de-AT", "pt-br", "pt-PT"]
            .map(|header| negotiated(request, extension, header, &offered, "en").unwrap());
        let tags: Vec<String> = ["de-AT", "zz"].map(str::to_owned).to_vec();
        let args = vec![strings(&tags), Value::Case("Lists".to_owned())];
        let resolved = block_on(request.call_values(extension, "resolveLocales", args))
            .expect("`resolveLocales` answers");
        (chosen, items(resolved.expect("the method returns a value")))
    });
    assert_eq!(chosen, ["de", "pt-BR", "en"]);
    assert_eq!(
        resolved,
        [
            Value::String("de".to_owned()),
            Value::String("und".to_owned())
        ]
    );
}

#[test]
fn an_empty_or_malformed_accept_language_returns_the_default() {
    let (chosen, offered_failure) = run(|request, extension| {
        let header = format!("{}de", "ja, ".repeat(32));
        let chosen = [
            "",
            " , ",
            "de;q=2",
            "de;x=1",
            "not a tag",
            "ja",
            "de;q=0",
            &header,
        ]
        .map(|header| negotiated(request, extension, header, &["en", "de"], "en").unwrap());
        let offered_failure = negotiated(request, extension, "de", &["de", "not a tag"], "en");
        (chosen, offered_failure)
    });
    assert_eq!(chosen, ["en"; 8], "the header never throws");
    assert!(
        matches!(offered_failure, Err(Failure::Error(Error::Invalid(_)))),
        "a malformed offered tag is the program's own: {offered_failure:?}"
    );
}
