//! The twelve default faces and the font context a layout shapes with
//! (`rule:core-classes/pdf-default-fonts`).
//!
//! Liberation Sans, Serif and Mono in regular, bold, italic and bold italic are embedded from
//! `extensions/fonts/`, the files the image component embeds too. `sans-serif`, Arial and
//! Helvetica are Sans, `serif`, Times and Times New Roman are Serif, and `monospace` and Courier
//! are Mono, compared without case. Any other name, and no name, is Sans.
//!
//! The font context is `parley`'s with `fontique`'s system discovery off: the collection holds
//! the twelve faces and nothing else, so a render on any machine shapes with the same fonts.
//!
//! Memory: the faces are part of the component's bytes and are never copied; a render's font
//! context holds `fontique`'s index of them, freed when the render returns.

use std::sync::Arc;

use parley::FontContext;
use parley::fontique::{Blob, Collection, CollectionOptions, SourceCache};

/// A default family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Sans,
    Serif,
    Mono,
}

impl Family {
    /// The family a CSS `font-family` name selects, quoted or not.
    pub fn from_css(name: &str) -> Family {
        let name = name.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        let is = |other: &str| name.eq_ignore_ascii_case(other);
        if is("serif") || is("times") || is("times new roman") || is("liberation serif") {
            Family::Serif
        } else if is("monospace") || is("courier") || is("courier new") || is("liberation mono") {
            Family::Mono
        } else {
            Family::Sans
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
