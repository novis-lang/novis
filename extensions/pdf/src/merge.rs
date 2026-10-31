//! Merging existing PDFs into one (`rule:core-classes/pdf-reader-and-merge`), through `krilla`'s
//! `pdf` feature.
//!
//! [`merge`] opens every part with [`crate::read::open`], so an encrypted part or one that does
//! not parse returns `Parse` before anything is written, and checks every page number against its
//! part's count, returning `Invalid` naming it. Then each chosen page is copied as a page of a new
//! document, in the order the parts and their lists give: a list that repeats a page repeats it,
//! and a list that leaves one out removes it. A part with no list gives every page.
//!
//! A copied page keeps its appearance and its resources. Its annotations, so its links and form
//! fields, are not copied, and neither are the parts' bookmarks, metadata or attachments. The new
//! file is PDF 2.0 when a part is, and PDF 1.7 otherwise, because `krilla` refuses a page newer
//! than the file it writes. A merge with no page returns `Invalid`.
//!
//! The whole merge runs under the job's [`Bound`] (`read`'s module doc), so every stream it
//! decodes, in [`open`] or while it copies a page, stops at the bound.
//!
//! Every byte of the output comes from the parts, so equal parts give byte-equal files.

use std::sync::Arc;

use hayro_interpret::hayro_syntax::PdfVersion as Source;
use krilla::configure::{ConfigurationBuilder, PdfVersion};
use krilla::pdf::PdfDocument;
use krilla::{Document, SerializeSettings};

use crate::Error;
use crate::read::{Bound, index, open, under};

/// The WIT `part`: a PDF, and the pages to take from it in order, counted from 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Part {
    pub data: Vec<u8>,
    pub pages: Option<Vec<u64>>,
}

/// One PDF of the chosen pages of `parts`, in order, each part opened under `bound`.
pub fn merge(parts: Vec<Part>, bound: Bound) -> Result<Vec<u8>, Error> {
    under(bound, || merge_parts(parts))
}

fn merge_parts(parts: Vec<Part>) -> Result<Vec<u8>, Error> {
    let mut chosen = Vec::with_capacity(parts.len());
    let mut newest = false;
    for part in parts {
        let pdf = open(part.data)?;
        let count = pdf.pages().len();
        let indices = match part.pages {
            Some(pages) => pages
                .into_iter()
                .map(|page| index(page, count))
                .collect::<Result<Vec<_>, _>>()?,
            None => (0..count).collect(),
        };
        newest |= matches!(pdf.version(), Source::Pdf20);
        chosen.push((PdfDocument::new(Arc::new(pdf)), indices));
    }
    if chosen.iter().all(|(_, indices)| indices.is_empty()) {
        return Err(Error::Invalid(
            "a merge needs at least one page, and the parts choose none".to_owned(),
        ));
    }
    let version = if newest {
        PdfVersion::Pdf20
    } else {
        PdfVersion::Pdf17
    };
    let configuration = ConfigurationBuilder::new()
        .with_version(version)
        .finish()
        .map_err(|err| Error::Runtime(format!("the merge could not be configured: {err:?}")))?;
    let mut document = Document::new_with(SerializeSettings {
        configuration,
        ..SerializeSettings::default()
    });
    for (pdf, indices) in &chosen {
        document.embed_pdf_pages(pdf, indices);
    }
    document
        .finish()
        .map_err(|err| Error::Runtime(format!("the merged PDF could not be written: {err}")))
}
