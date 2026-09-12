//! PER's base style, as `rule:tooling/fmt-base-style-is-per` states it.
//!
//! Each case is one mangled file and the canonical text it formats to, so what
//! a failure prints is the whole disagreement rather than a property that went
//! false somewhere. An input is wrong about the rule under test and about
//! nothing else — every other byte of it is what the printer answers today, so
//! a case fails for its own reason and an unlanded rule cannot mask it.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn indentation_is_four_spaces_per_block_depth() {
    let mangled = "\
<?nvs
class Tag {
      public string $name;

  public function rename(string $name): void {
$this->name = $name;
        if ($name == \"\") {
  $this->name = \"untitled\";
            }
  }
}

  var $tag = new Tag(\"red\");
$tag->rename(\"\");
";
    let canonical = "\
<?nvs
class Tag {
    public string $name;

    public function rename(string $name): void {
        $this->name = $name;
        if ($name == \"\") {
            $this->name = \"untitled\";
        }
    }
}

var $tag = new Tag(\"red\");
$tag->rename(\"\");
";
    assert_eq!(formatted(mangled), canonical);
}
