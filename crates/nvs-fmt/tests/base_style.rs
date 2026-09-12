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
class Tag
{
      public string $name;

  public function rename(string $name): void
    {
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
class Tag
{
    public string $name;

    public function rename(string $name): void
    {
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

#[test]
fn modifiers_are_written_in_the_canonical_order() {
    let mangled = "\
<?nvs
abstract class Cache
{
    static private int $hits = 0;
    readonly private string $name;

    public abstract function warm(): void;

    final public function reset(): void
    {
    }
}
";
    let canonical = "\
<?nvs
abstract class Cache
{
    private static int $hits = 0;
    private readonly string $name;

    abstract public function warm(): void;

    final public function reset(): void
    {
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r() {
    let mangled = "\
<?nvs
class Queue {
    public function drain(int $limit): void {
        for ($i = 0; $i < $limit; $i++)
        {
            if ($i == 0)
            {
                continue;
            }
            elseif ($i == 1)
            {
                break;
            }
            else
            {
                echo $i;
            }
        }
        try
        {
            echo \"done\";
        }
        catch (Error $e)
        {
            echo \"failed\";
        }
    }
}
";
    let canonical = "\
<?nvs
class Queue
{
    public function drain(int $limit): void
    {
        for ($i = 0; $i < $limit; $i++) {
            if ($i == 0) {
                continue;
            } elseif ($i == 1) {
                break;
            } else {
                echo $i;
            }
        }
        try {
            echo \"done\";
        } catch (Error $e) {
            echo \"failed\";
        }
    }
}
";
    assert_eq!(formatted(mangled), canonical);
}
