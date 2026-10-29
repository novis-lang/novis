//! The registry's advisory feed (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`):
//! every retraction and every withhold in the log, in one signed file a client reads in one fetch.
//!
//! The feed is the file [`FILE`] in the log's directory, a signed note whose text is its JSON on
//! one line, signed as an index file is ([`index::sign`]): `schema`, `advisories`, `signed_at`
//! and `checkpoint`, the size and root hash of the log it lists. Each advisory is `log`, the
//! number of its record, and `entry`, that record's entry. The rule's fragment states the schema.
//!
//! **The feed is checked against the log, never trusted alone.** [`Feed::open`] needs the log's
//! pinned key. [`Feed::against`] then walks every record of the log up to the feed's size, and
//! a retraction or a withhold the feed does not list is an error, as is a listed record that the
//! log has with other bytes. A feed that hides an advisory therefore disagrees with the log. Only
//! [`is_advisory`]'s kinds are in the feed: an unlist is not an advisory.
//!
//! **Memory:** an opened feed holds every advisory the registry ever logged, and the walk holds
//! one record at a time beside it. Nothing here runs on the request path.

use serde::Deserialize;
use serde_json::Value;

use super::entry::Entry;
use super::index::{self, CheckpointLine, Stamp};
use super::note::{self, Checkpoint, Signer, Verifier};

/// The feed's file name in the log's directory, beside `checkpoint`.
pub(crate) const FILE: &str = "advisories";

/// The schema version of the feed this client reads.
pub(crate) const SCHEMA: u64 = 1;

/// The feed, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FeedFile {
    schema: u64,
    advisories: Vec<Line>,
    signed_at: u64,
    checkpoint: CheckpointLine,
}

/// One advisory in the feed, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    log: u64,
    entry: Value,
}

/// One advisory: a log record that retracts or withholds a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Advisory {
    /// The number of its record in the log.
    pub(crate) log: u64,
    /// The record's entry.
    pub(crate) entry: Entry,
    /// The entry's canonical bytes, which the log's record must be.
    record: Vec<u8>,
}

/// An advisory feed whose signature checked.
#[derive(Debug)]
pub(crate) struct Feed {
    /// When it was signed, and the log it lists.
    pub(crate) stamp: Stamp,
    /// Every advisory, in the log's order.
    pub(crate) advisories: Vec<Advisory>,
}

impl Advisory {
    /// Record `log` of the log, whose bytes are `record`, as an advisory, or `None` when it is
    /// not one. A record past a feed's size is read this way.
    pub(crate) fn from_record(log: u64, record: &[u8]) -> Option<Self> {
        let entry = Entry::parse(record).ok().filter(is_advisory)?;
        Some(Self {
            log,
            entry,
            record: record.to_vec(),
        })
    }
}

/// Whether the feed lists `entry`: a retraction or a withhold.
pub(crate) fn is_advisory(entry: &Entry) -> bool {
    matches!(entry, Entry::Retract { .. } | Entry::Withhold { .. })
}

/// The feed of `advisories`, each a record's number and its bytes, signed by `signer` at `at`
/// with `checkpoint`.
///
/// # Errors
///
/// A record is not JSON, or the note cannot be signed.
pub(crate) fn sign(
    advisories: &[(u64, Vec<u8>)],
    signer: &Signer,
    checkpoint: &Checkpoint,
    at: u64,
) -> Result<String, String> {
    let mut lines = Vec::with_capacity(advisories.len());
    for (log, record) in advisories {
        let entry: Value = serde_json::from_slice(record)
            .map_err(|err| format!("record {log} is not JSON: {err}"))?;
        lines.push(serde_json::json!({ "log": log, "entry": entry }));
    }
    let json = serde_json::json!({ "schema": SCHEMA, "advisories": lines });
    index::sign(json.to_string().as_bytes(), signer, checkpoint, at)
}

impl Feed {
    /// The feed `bytes`, once it opens with `verifier`, is signed at most [`index::AHEAD`] past
    /// `now`, and lists only advisories, each once, in the log's order, inside the log it names.
    ///
    /// # Errors
    ///
    /// The signature, the schema or one advisory is wrong. The message says which.
    pub(crate) fn open(bytes: &[u8], verifier: &Verifier, now: u64) -> Result<Self, String> {
        let opened = note::open(bytes, std::slice::from_ref(verifier)).map_err(|err| {
            format!("its signature does not check with the registry's key: {err}")
        })?;
        let file: FeedFile = serde_json::from_str(opened.text)
            .map_err(|err| format!("it is not an advisory feed: {err}"))?;
        if file.schema != SCHEMA {
            return Err(format!(
                "its schema is {}, and this client reads {SCHEMA}",
                file.schema
            ));
        }
        let stamp = Stamp::read(file.signed_at, &file.checkpoint, now)?;
        let mut advisories: Vec<Advisory> = Vec::with_capacity(file.advisories.len());
        for line in file.advisories {
            let n = line.log;
            if n >= stamp.size {
                return Err(format!(
                    "it lists record {n} of a log of {} entries",
                    stamp.size
                ));
            }
            if advisories.last().is_some_and(|last| n <= last.log) {
                return Err(format!("record {n} is out of the log's order"));
            }
            let json = serde_json::to_vec(&line.entry).expect("a JSON value serializes");
            let entry = Entry::parse(&json).map_err(|why| format!("record {n}: {why}"))?;
            if !is_advisory(&entry) {
                return Err(format!("record {n} is not a retraction or a withhold"));
            }
            let record = entry.canonical();
            advisories.push(Advisory {
                log: n,
                entry,
                record,
            });
        }
        Ok(Self { stamp, advisories })
    }

    /// The check of this feed against its log, given every record in order.
    pub(crate) fn against(&self) -> Against<'_> {
        Against {
            feed: self,
            next: 0,
        }
    }
}

/// The check of a feed against the log, one record at a time ([`Feed::against`]).
pub(crate) struct Against<'a> {
    feed: &'a Feed,
    /// The first advisory no record has matched yet.
    next: usize,
}

impl Against<'_> {
    /// Checks record `n`, whose bytes are `record`. The records come in order from 0, and one
    /// at or past the feed's size is not checked.
    ///
    /// # Errors
    ///
    /// The log has an advisory at `n` the feed does not list, or the feed lists record `n` with
    /// other bytes or as an advisory the log does not have.
    pub(crate) fn record(&mut self, n: u64, record: &[u8]) -> Result<(), String> {
        if n >= self.feed.stamp.size {
            return Ok(());
        }
        let listed = self
            .feed
            .advisories
            .get(self.next)
            .filter(|advisory| advisory.log == n);
        let logged = Entry::parse(record).is_ok_and(|entry| is_advisory(&entry));
        match (listed, logged) {
            (Some(advisory), true) if advisory.record == record => {
                self.next += 1;
                Ok(())
            }
            (Some(_), true) => Err(format!(
                "the advisory feed lists record {n} with other bytes than the log has"
            )),
            (Some(_), false) => Err(format!(
                "the advisory feed lists record {n}, and the log has no advisory there"
            )),
            (None, true) => Err(format!(
                "the advisory feed does not list record {n}, an advisory in the log: {}",
                String::from_utf8_lossy(record)
            )),
            (None, false) => Ok(()),
        }
    }

    /// Ends the check, once every record up to the feed's size was given.
    ///
    /// # Errors
    ///
    /// The feed lists a record the walk never reached.
    pub(crate) fn finish(self) -> Result<(), String> {
        match self.feed.advisories.get(self.next) {
            Some(advisory) => Err(format!(
                "the advisory feed lists record {}, which the log does not have",
                advisory.log
            )),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::log::{self, HashReader};

    const NOW: u64 = 1_800_000_000;

    fn signer() -> Signer {
        Signer::parse(
            "PRIVATE+KEY+PeterNeumann+c74f20a3+AYEKFALVFGyNhPJEMzD1QIDr+Y7hfZx09iUvxdXHKDFz",
        )
        .expect("Go's test key parses")
    }

    fn records() -> Vec<Vec<u8>> {
        [
            r#"{"v":1,"kind":"unlist","package":"acme/http","version":"1.0.0"}"#,
            r#"{"v":1,"kind":"retract","package":"acme/http","version":"1.1.0","reason":"It deletes files.","fixed_in":"1.1.1"}"#,
            r#"{"v":1,"kind":"withhold","package":"acme/http","version":"1.2.0","reason":"Malware."}"#,
        ]
        .iter()
        .map(|json| Entry::parse(json.as_bytes()).unwrap().canonical())
        .collect()
    }

    /// The tree of `records` in memory, as its stored hashes.
    struct Memory(Vec<log::Hash>);

    impl HashReader for Memory {
        fn read_hashes(&mut self, indexes: &[u64]) -> Result<Vec<log::Hash>, log::Error> {
            Ok(indexes
                .iter()
                .map(|i| self.0[usize::try_from(*i).unwrap()])
                .collect())
        }
    }

    fn checkpoint(records: &[Vec<u8>]) -> Checkpoint {
        let mut memory = Memory(Vec::new());
        for (n, record) in (0..).zip(records) {
            let hashes = log::stored_hashes(n, record, &mut memory).unwrap();
            memory.0.extend(hashes);
        }
        let size = records.len() as u64;
        let root = log::tree_hash(size, &mut memory).unwrap();
        Checkpoint::new("PeterNeumann", size, root).unwrap()
    }

    fn check(feed: &Feed, records: &[Vec<u8>]) -> Result<(), String> {
        let mut against = feed.against();
        for (n, record) in (0..).zip(records) {
            against.record(n, record)?;
        }
        against.finish()
    }

    #[test]
    fn a_feed_of_every_advisory_opens_and_agrees_with_the_log() {
        let records = records();
        let listed = vec![(1, records[1].clone()), (2, records[2].clone())];
        let note = sign(&listed, &signer(), &checkpoint(&records), NOW).unwrap();
        let feed = Feed::open(note.as_bytes(), &signer().verifier(), NOW).unwrap();
        assert_eq!(feed.advisories.len(), 2);
        assert_eq!(feed.stamp.size, 3);
        check(&feed, &records).unwrap();
    }

    #[test]
    fn a_feed_that_hides_an_advisory_disagrees_with_the_log() {
        let records = records();
        let listed = vec![(2, records[2].clone())];
        let note = sign(&listed, &signer(), &checkpoint(&records), NOW).unwrap();
        let feed = Feed::open(note.as_bytes(), &signer().verifier(), NOW).unwrap();
        let err = check(&feed, &records).unwrap_err();
        assert!(err.contains("does not list record 1"), "{err}");
    }

    #[test]
    fn a_listed_record_the_log_has_otherwise_is_an_error() {
        let records = records();
        let mut changed = records.clone();
        changed[1] = Entry::parse(
            br#"{"v":1,"kind":"retract","package":"acme/http","version":"1.1.0","reason":"It deletes files."}"#,
        )
        .unwrap()
        .canonical();
        let listed = vec![(1, changed[1].clone()), (2, records[2].clone())];
        let note = sign(&listed, &signer(), &checkpoint(&records), NOW).unwrap();
        let feed = Feed::open(note.as_bytes(), &signer().verifier(), NOW).unwrap();
        let err = check(&feed, &records).unwrap_err();
        assert!(err.contains("other bytes"), "{err}");
    }

    #[test]
    fn a_feed_that_lists_what_is_not_an_advisory_does_not_open() {
        let records = records();
        let listed = vec![(0, records[0].clone())];
        let note = sign(&listed, &signer(), &checkpoint(&records), NOW).unwrap();
        let err = Feed::open(note.as_bytes(), &signer().verifier(), NOW).unwrap_err();
        assert!(err.contains("not a retraction or a withhold"), "{err}");
    }
}
