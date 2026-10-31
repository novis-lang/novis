//! The Factur-X invoice: one XML file attached to a PDF/A-3b document
//! (`rule:core-classes/pdf-factur-x-is-the-one-attachment`).
//!
//! [`check`] parses the XML with no DTD, so no entity is ever expanded, and returns `Parse` for
//! one that is not well-formed. It reads one thing from the invoice: the guideline id of its
//! `GuidelineSpecifiedDocumentContextParameter`, which gives the [`Level`] the metadata names.
//! An invoice with no guideline id the standard lists is `EN 16931`.
//!
//! **An invoice nests at most [`MAX_DEPTH`] elements**, and [`check`] returns `Parse` for a
//! deeper one. A real invoice nests about ten. `roxmltree`'s parse keeps no stack of its own,
//! so the depth is counted after it, in one pass over the node tree. [`depth_error`] is that
//! count alone, and `read` applies it to a received invoice that parses, so neither job hands a
//! program XML deeper than any invoice.
//!
//! [`embed`] attaches the XML as `factur-x.xml`, `text/xml`, dated at the document's creation
//! time, with the `AFRelationship` the level fixes: `Data` for `MINIMUM` and `BASIC WL`, which
//! are not whole invoices, and `Alternative` for the rest. `krilla` lists it in the catalog's
//! `/AF` and its embedded-file name tree.
//!
//! [`xmp`] adds the standard's XMP fields to the finished file, because `krilla` 0.8 has no way
//! to add an XMP property. Its metadata stream is the last packet in the file, unfiltered, and
//! is followed only by the catalog, the cross-reference table and the trailer. The rewrite adds
//! the `fx` extension schema to the `pdfaExtension:schemas` bag PDF/A-3b already writes, and a
//! description holding `fx:DocumentType`, `fx:DocumentFileName`, `fx:Version` and
//! `fx:ConformanceLevel`. It then writes the stream's new `/Length` and moves every
//! cross-reference offset after the packet, and `startxref`, by the bytes added. The file's `/ID`
//! stays the hash `krilla` took before the rewrite, so equal input is still byte-equal output.
//!
//! Memory: the XML's node tree while [`check`] runs, and one copy of the file while [`xmp`]
//! rewrites it, both inside the request's cap.

use krilla::Data;
use krilla::Document;
use krilla::embed::{AssociationKind, EmbeddedFile, MimeType};
use krilla::metadata::DateTime;

use crate::Error;

/// The name the standard gives the attached XML.
pub const FILE_NAME: &str = "factur-x.xml";

/// How many elements deep an invoice may nest.
pub const MAX_DEPTH: usize = 256;

/// The namespace of the Factur-X XMP fields.
const NAMESPACE: &str = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/// A Factur-X profile, as `fx:ConformanceLevel` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Minimum,
    BasicWl,
    Basic,
    En16931,
    Extended,
    XRechnung,
}

impl Level {
    /// The level's name in the XMP.
    pub fn name(self) -> &'static str {
        match self {
            Level::Minimum => "MINIMUM",
            Level::BasicWl => "BASIC WL",
            Level::Basic => "BASIC",
            Level::En16931 => "EN 16931",
            Level::Extended => "EXTENDED",
            Level::XRechnung => "XRECHNUNG",
        }
    }

    /// The level whose guideline id is `id`.
    fn of(id: &str) -> Level {
        let id = id.trim();
        if id.contains("xrechnung") {
            Level::XRechnung
        } else if id.ends_with(":extended") {
            Level::Extended
        } else if id.ends_with(":basicwl") {
            Level::BasicWl
        } else if id.ends_with(":basic") {
            Level::Basic
        } else if id.ends_with(":minimum") {
            Level::Minimum
        } else {
            Level::En16931
        }
    }
}

/// The level of the invoice `xml`. An XML that is not well-formed, has a DTD, or nests more than
/// [`MAX_DEPTH`] elements returns `Parse`.
pub fn check(xml: &str) -> Result<Level, Error> {
    let tree = parse(xml)
        .map_err(|err| Error::Parse(format!("the invoice XML is not well-formed: {err}")))?;
    if let Some(err) = too_deep(&tree) {
        return Err(err);
    }
    let id = tree
        .descendants()
        .find(|node| node.tag_name().name() == "GuidelineSpecifiedDocumentContextParameter")
        .and_then(|node| {
            node.children()
                .find(|child| child.tag_name().name() == "ID")
        })
        .and_then(|node| node.text());
    Ok(id.map_or(Level::En16931, Level::of))
}

/// `Parse` for invoice `xml` that parses and nests more than [`MAX_DEPTH`] elements, and `None`
/// for any other.
pub fn depth_error(xml: &str) -> Option<Error> {
    parse(xml).ok().and_then(|tree| too_deep(&tree))
}

/// `xml` parsed with no DTD.
fn parse(xml: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: false,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(xml, options)
}

/// `Parse` when `tree` nests more than [`MAX_DEPTH`] elements. Nodes come in document order, so
/// each node's parent is counted before it.
fn too_deep(tree: &roxmltree::Document<'_>) -> Option<Error> {
    let mut depths = vec![0_u32; tree.descendants().count()];
    for node in tree.root().descendants().skip(1) {
        let parent = node
            .parent()
            .map_or(0, |parent| depths[parent.id().get_usize()]);
        let depth = parent + u32::from(node.is_element());
        if depth as usize > MAX_DEPTH {
            return Some(Error::Parse(format!(
                "the invoice XML nests elements more than {MAX_DEPTH} levels deep"
            )));
        }
        depths[node.id().get_usize()] = depth;
    }
    None
}

/// Attaches `xml` to `document` as its Factur-X invoice of `level`, dated `created`.
pub fn embed(document: &mut Document, xml: &str, level: Level, created: Option<DateTime>) {
    let association_kind = match level {
        Level::Minimum | Level::BasicWl => AssociationKind::Data,
        _ => AssociationKind::Alternative,
    };
    // The document has no other attachment, so the name is never taken.
    let _ = document.embed_file(EmbeddedFile {
        path: FILE_NAME.to_owned(),
        mime_type: MimeType::new("text/xml"),
        description: Some("The Factur-X invoice of this document".to_owned()),
        association_kind,
        data: Data::from(xml.as_bytes().to_vec()),
        modification_date: created,
        compress: Some(true),
        location: None,
    });
}

/// `file` with the Factur-X XMP fields of an invoice of `level` in its metadata stream.
pub fn xmp(file: Vec<u8>, level: Level) -> Result<Vec<u8>, Error> {
    let lost = |what: &str| Error::Runtime(format!("the written file has no {what}"));
    let begin = rfind(&file, b"<?xpacket begin=", file.len()).ok_or_else(|| lost("XMP packet"))?;
    let end_marker = b"<?xpacket end=\"r\"?>";
    let end =
        find(&file, end_marker, begin).ok_or_else(|| lost("XMP packet end"))? + end_marker.len();
    let packet = std::str::from_utf8(&file[begin..end]).map_err(|_| lost("UTF-8 XMP packet"))?;

    let bag = "</rdf:Bag></pdfaExtension:schemas>";
    let at_bag = packet
        .rfind(bag)
        .ok_or_else(|| lost("PDF/A extension schemas"))?;
    let rdf = "</rdf:RDF>";
    let at_rdf = packet.rfind(rdf).ok_or_else(|| lost("RDF element"))?;
    let mut new_packet = String::with_capacity(packet.len() + 2048);
    new_packet.push_str(&packet[..at_bag]);
    new_packet.push_str(&schema());
    new_packet.push_str(&packet[at_bag..at_rdf]);
    new_packet.push_str(&format!(
        "<rdf:Description rdf:about=\"\" xmlns:fx=\"{NAMESPACE}\">\
         <fx:DocumentType>INVOICE</fx:DocumentType>\
         <fx:DocumentFileName>{FILE_NAME}</fx:DocumentFileName>\
         <fx:Version>1.0</fx:Version>\
         <fx:ConformanceLevel>{}</fx:ConformanceLevel></rdf:Description>",
        level.name()
    ));
    new_packet.push_str(&packet[at_rdf..]);

    // The stream's dictionary is between its object's header and the packet.
    let header = rfind(&file, b" obj", begin).ok_or_else(|| lost("metadata object"))?;
    let key = find(&file[..begin], b"/Length ", header).ok_or_else(|| lost("metadata length"))?
        + b"/Length ".len();
    let digits = file[key..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();

    let mut out = Vec::with_capacity(file.len() + new_packet.len() - packet.len() + 8);
    out.extend_from_slice(&file[..key]);
    out.extend_from_slice(new_packet.len().to_string().as_bytes());
    out.extend_from_slice(&file[key + digits..begin]);
    out.extend_from_slice(new_packet.as_bytes());
    let delta = out.len() - end;
    out.extend_from_slice(&file[end..]);

    // The cross-reference table: `xref`, `0 N`, then N entries of 20 bytes each.
    let start = rfind(&out, b"startxref\n", out.len()).ok_or_else(|| lost("startxref"))?;
    let number = start + b"startxref\n".len();
    let width = out[number..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let old: usize = std::str::from_utf8(&out[number..number + width])
        .ok()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| lost("cross-reference offset"))?;
    let xref = old + delta;
    if !out[xref..].starts_with(b"xref\n0 ") {
        return Err(lost("cross-reference table"));
    }
    let count_at = xref + b"xref\n0 ".len();
    let count_len = out[count_at..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let count: usize = std::str::from_utf8(&out[count_at..count_at + count_len])
        .ok()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| lost("cross-reference count"))?;
    let entries = count_at + count_len + 1;
    for index in 0..count {
        let entry = entries + index * 20;
        if out.get(entry + 17) != Some(&b'n') {
            continue;
        }
        let offset: usize = std::str::from_utf8(&out[entry..entry + 10])
            .ok()
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| lost("cross-reference entry"))?;
        if offset > begin {
            let moved = format!("{:010}", offset + delta);
            out[entry..entry + 10].copy_from_slice(moved.as_bytes());
        }
    }
    out.truncate(number);
    out.extend_from_slice(xref.to_string().as_bytes());
    out.extend_from_slice(b"\n%%EOF");
    Ok(out)
}

/// The `fx` schema's entry in `pdfaExtension:schemas`.
fn schema() -> String {
    let mut out = format!(
        "<rdf:li rdf:parseType=\"Resource\" \
         xmlns:pdfaSchema=\"http://www.aiim.org/pdfa/ns/schema#\" \
         xmlns:pdfaProperty=\"http://www.aiim.org/pdfa/ns/property#\">\
         <pdfaSchema:schema>Factur-X PDFA Extension Schema</pdfaSchema:schema>\
         <pdfaSchema:namespaceURI>{NAMESPACE}</pdfaSchema:namespaceURI>\
         <pdfaSchema:prefix>fx</pdfaSchema:prefix><pdfaSchema:property><rdf:Seq>"
    );
    for (name, description) in [
        ("DocumentFileName", "The name of the embedded XML document"),
        (
            "DocumentType",
            "The type of the hybrid document in capital letters, e.g. INVOICE or ORDER",
        ),
        (
            "Version",
            "The actual version of the standard applying to the embedded XML document",
        ),
        (
            "ConformanceLevel",
            "The conformance level of the embedded XML document",
        ),
    ] {
        out.push_str(&format!(
            "<rdf:li rdf:parseType=\"Resource\">\
             <pdfaProperty:name>{name}</pdfaProperty:name>\
             <pdfaProperty:valueType>Text</pdfaProperty:valueType>\
             <pdfaProperty:category>external</pdfaProperty:category>\
             <pdfaProperty:description>{description}</pdfaProperty:description></rdf:li>"
        ));
    }
    out.push_str("</rdf:Seq></pdfaSchema:property></rdf:li>");
    out
}

/// Where `needle` first occurs in `haystack` at or after `from`.
fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

/// Where `needle` last occurs in `haystack` before `before`.
fn rfind(haystack: &[u8], needle: &[u8], before: usize) -> Option<usize> {
    haystack
        .get(..before)?
        .windows(needle.len())
        .rposition(|window| window == needle)
}
