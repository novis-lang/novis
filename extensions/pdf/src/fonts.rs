//! The twelve default faces and the font context a layout shapes with
//! (`rule:core-classes/pdf-default-fonts`).
//!
//! Liberation Sans, Serif and Mono in regular, bold, italic and bold italic are embedded from
//! `extensions/fonts/`, the files the image component embeds too. `sans-serif`, Arial and
//! Helvetica are Sans, `serif`, Times and Times New Roman are Serif, and `monospace` and Courier
//! are Mono, compared without case. Any other name, and no name, is Sans.
//!
//! The font context is `parley`'s with `fontique`'s system discovery off: the collection holds
//! the twelve faces and the document's `@font-face` fonts and nothing else, so a render on any
//! machine shapes with the same fonts. [`Fonts::new`] registers each `@font-face` under the
//! family name its rule gives, with its `font-weight` and `font-style`, from the first `src` URL
//! the asset map has. A file that is not a font returns `Parse` naming it.
//!
//! [`Fonts::chain`] is a `font-family` list as the fonts a run tries in order: each name an
//! `@font-face` rule registered, then the default its first generic or known name maps to, and
//! Liberation Sans when no name maps to a default. A name that is neither is skipped.
//!
//! Memory: the default faces are part of the component's bytes and are never copied. The first
//! SVG with `<text>` builds [`svg_database`]'s index of them once per instance. A render
//! holds one copy of each `@font-face` file and `fontique`'s index of every face, freed when the
//! render returns.

use std::sync::{Arc, OnceLock};

use parley::FontContext;
use parley::fontique::{
    Blob, Collection, CollectionOptions, FontInfoOverride, FontStyle, FontWeight, SourceCache,
};

use crate::Error;
use crate::assets::Assets;
use crate::css::value::{Comp, components};

/// A default family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Sans,
    Serif,
    Mono,
}

impl Family {
    /// The family a CSS `font-family` name selects, quoted or not. Any other name is Sans.
    pub fn from_css(name: &str) -> Family {
        Family::known(name).unwrap_or(Family::Sans)
    }

    /// The family a generic or known CSS `font-family` name maps to, and `None` for any other.
    pub fn known(name: &str) -> Option<Family> {
        let name = name.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        let is = |other: &str| name.eq_ignore_ascii_case(other);
        if is("serif") || is("times") || is("times new roman") || is("liberation serif") {
            Some(Family::Serif)
        } else if is("monospace") || is("courier") || is("courier new") || is("liberation mono") {
            Some(Family::Mono)
        } else if is("sans-serif")
            || is("arial")
            || is("helvetica")
            || is("liberation sans")
            || is("system-ui")
        {
            Some(Family::Sans)
        } else {
            None
        }
    }

    /// The name the family is registered under in the font context.
    pub fn registered_name(self) -> &'static str {
        match self {
            Family::Sans => "Liberation Sans",
            Family::Serif => "Liberation Serif",
            Family::Mono => "Liberation Mono",
        }
    }
}

macro_rules! liberation {
    ($name:literal) => {
        include_bytes!(concat!("../../fonts/Liberation", $name, ".ttf")).as_slice()
    };
}

/// The twelve faces, in a fixed order: family, then regular, bold, italic, bold italic.
pub static FACES: [&[u8]; 12] = [
    liberation!("Sans-Regular"),
    liberation!("Sans-Bold"),
    liberation!("Sans-Italic"),
    liberation!("Sans-BoldItalic"),
    liberation!("Serif-Regular"),
    liberation!("Serif-Bold"),
    liberation!("Serif-Italic"),
    liberation!("Serif-BoldItalic"),
    liberation!("Mono-Regular"),
    liberation!("Mono-Bold"),
    liberation!("Mono-Italic"),
    liberation!("Mono-BoldItalic"),
];

/// The face of `family` in the given weight and slant.
pub fn face(family: Family, bold: bool, italic: bool) -> &'static [u8] {
    let base = match family {
        Family::Sans => 0,
        Family::Serif => 4,
        Family::Mono => 8,
    };
    FACES[base + usize::from(bold) + 2 * usize::from(italic)]
}

/// Which of [`FACES`] `data` is, by address: the font context's blobs borrow the embedded
/// bytes, so a shaped run's font is found without hashing it.
pub fn face_index(data: &[u8]) -> Option<usize> {
    FACES
        .iter()
        .position(|face| face.as_ptr() == data.as_ptr() && face.len() == data.len())
}

/// The font database `usvg` lays out an SVG's `<text>` with: the twelve faces and no system
/// font, `serif`, `sans-serif` and `monospace` mapped to Liberation, and any other family
/// name falling back to Liberation Serif, as `usvg` falls back to `serif`. It is built on first
/// use and shared, and it borrows the embedded bytes.
pub fn svg_database() -> Arc<usvg::fontdb::Database> {
    static DATABASE: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let database = DATABASE.get_or_init(|| {
        let mut database = usvg::fontdb::Database::new();
        for face in FACES {
            database.load_font_source(usvg::fontdb::Source::Binary(Arc::new(face)));
        }
        database.set_serif_family(Family::Serif.registered_name());
        database.set_sans_serif_family(Family::Sans.registered_name());
        database.set_monospace_family(Family::Mono.registered_name());
        Arc::new(database)
    });
    Arc::clone(database)
}

/// A font context that holds the twelve faces and no system font.
pub fn context() -> FontContext {
    let mut collection = Collection::new(CollectionOptions {
        shared: false,
        system_fonts: false,
    });
    for face in FACES {
        collection.register_fonts(Blob::new(Arc::new(face)), None);
    }
    FontContext {
        collection,
        source_cache: SourceCache::default(),
    }
}

/// A face's bytes, shared by the font context and the writer.
pub type FaceData = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// A render's fonts: the font context, and every face it can shape with, the twelve defaults
/// first in [`FACES`]' order.
pub struct Fonts {
    pub context: FontContext,
    pub faces: Vec<FaceData>,
    /// The family names `@font-face` registered, in lower case.
    families: Vec<String>,
}

impl Fonts {
    /// The defaults and the `@font-face` rules' fonts, as [`crate::css::Styles::font_faces`]
    /// gives them.
    pub fn new(font_faces: &[Vec<(String, String)>], assets: &Assets<'_>) -> Result<Fonts, Error> {
        let mut context = context();
        let mut faces: Vec<FaceData> = FACES.iter().map(|f| Arc::new(*f) as FaceData).collect();
        let mut families = Vec::new();
        for rule in font_faces {
            let get = |name: &str| {
                rule.iter()
                    .rev()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v.as_str())
            };
            let (Some(family), Some(src)) = (get("font-family"), get("src")) else {
                continue;
            };
            let family = family.trim().trim_matches(|c| c == '"' || c == '\'');
            let Some(url) = components(src).into_iter().flatten().find_map(|c| match c {
                Comp::Url(url) => Some(url),
                Comp::Function(name, args) if name == "url" => Some(
                    args.trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_owned(),
                ),
                _ => None,
            }) else {
                continue;
            };
            let bytes = assets.resolve(&url)?.into_owned();
            let weight = match get("font-weight").map(str::trim) {
                Some("bold") => 700.0,
                Some(w) => w.parse().unwrap_or(400.0),
                None => 400.0,
            };
            let style = match get("font-style").map(str::trim) {
                Some("italic") | Some("oblique") => FontStyle::Italic,
                _ => FontStyle::Normal,
            };
            let data: FaceData = Arc::new(bytes);
            let registered = context.collection.register_fonts(
                Blob::new(data.clone()),
                Some(FontInfoOverride {
                    family_name: Some(family),
                    weight: Some(FontWeight::new(weight)),
                    style: Some(style),
                    ..FontInfoOverride::default()
                }),
            );
            if registered.is_empty() {
                return Err(Error::Parse(format!("`{url}` is not a font")));
            }
            faces.push(data);
            families.push(family.to_ascii_lowercase());
        }
        Ok(Fonts {
            context,
            faces,
            families,
        })
    }

    /// The fonts a `font-family` list tries, by their registered names.
    pub fn chain(&self, names: &[String]) -> Vec<String> {
        let mut chain = Vec::new();
        for name in names {
            let bare = name.trim().trim_matches(|c| c == '"' || c == '\'');
            if self.families.contains(&bare.to_ascii_lowercase()) {
                chain.push(bare.to_owned());
            } else if let Some(family) = Family::known(bare) {
                chain.push(family.registered_name().to_owned());
                return chain;
            }
        }
        chain.push(Family::Sans.registered_name().to_owned());
        chain
    }

    /// Which of [`Fonts::faces`] `data` is, by address.
    pub fn face_index(&self, data: &[u8]) -> Option<usize> {
        self.faces.iter().position(|face| {
            let face = (**face).as_ref();
            face.as_ptr() == data.as_ptr() && face.len() == data.len()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_names_map_to_their_family() {
        assert_eq!(Family::from_css("Arial"), Family::Sans);
        assert_eq!(Family::from_css("\"Times New Roman\""), Family::Serif);
        assert_eq!(Family::from_css("COURIER"), Family::Mono);
        assert_eq!(Family::from_css("sans-serif"), Family::Sans);
        assert_eq!(Family::from_css("fantasy"), Family::Sans);
    }

    #[test]
    fn each_face_is_found_by_its_address() {
        for (at, face) in FACES.iter().enumerate() {
            assert_eq!(face_index(face), Some(at));
        }
        assert_eq!(face(Family::Serif, true, true).as_ptr(), FACES[7].as_ptr());
    }
}
