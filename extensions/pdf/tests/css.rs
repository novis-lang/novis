//! The cascade (`rule:core-classes/pdf-css-subset`): what decides an element's computed style,
//! that every property `subset.json` lists parses, and that everything else is dropped and
//! reported with its selector and position.

use nvs_pdf::css::{self, Color, Length, Value, properties};
use nvs_pdf::{Document, assets::Assets, parse};

fn cascade(html: &str) -> (parse::Dom, css::Styles) {
    let dom = parse::parse(html);
    let styles = css::cascade(&dom, html, &Assets::new(&[]));
    (dom, styles)
}

fn by_id(dom: &parse::Dom, id: &str) -> usize {
    dom.nodes
        .iter()
        .position(|node| node.attr("id") == Some(id))
        .unwrap_or_else(|| panic!("no element has the id {id}"))
}

fn get<'a>(
    dom: &parse::Dom,
    styles: &'a css::Styles,
    id: &str,
    property: &str,
) -> Option<&'a Value> {
    styles
        .of(by_id(dom, id))
        .expect("an element has a style")
        .get(property)
}

fn rgb(r: u8, g: u8, b: u8) -> Value {
    Value::Color(Color { r, g, b, a: 1.0 })
}

fn pt(pt: f32) -> Value {
    Value::Length(Length { pt, percent: 0.0 })
}

#[test]
fn specificity_inheritance_and_important_decide_the_computed_style() {
    let html = r#"<style>
        p { color: red; font-size: 10pt }
        div p { color: purple }
        .note { color: green }
        #lead, #lead2 { color: blue }
        .strong { color: black !important }
        #loud { color: black !important }
        section { font-size: 20pt; color: orange }
    </style>
    <section>
      <p id="lead" class="note">a</p>
      <p id="lead2" style="color: white">b</p>
      <p id="b" class="note strong" style="color: white">c</p>
      <p id="loud" style="color: white !important">d</p>
      <p id="c">e</p>
      <span id="d">f</span>
      <div id="e" style="font-size: 0.5em; margin-top: 2em; line-height: 1.5"><i id="f">g</i></div>
    </section>"#;
    let (dom, styles) = cascade(html);
    assert!(styles.dropped.is_empty(), "{:?}", styles.dropped);
    // An id beats a class, and a class beats two type selectors.
    assert_eq!(get(&dom, &styles, "lead", "color"), Some(&rgb(0, 0, 255)));
    // A `style` attribute beats an id.
    assert_eq!(
        get(&dom, &styles, "lead2", "color"),
        Some(&rgb(255, 255, 255))
    );
    // `!important` in a sheet beats a `style` attribute, and `!important` in the attribute
    // beats `!important` in a sheet.
    assert_eq!(get(&dom, &styles, "b", "color"), Some(&rgb(0, 0, 0)));
    assert_eq!(
        get(&dom, &styles, "loud", "color"),
        Some(&rgb(255, 255, 255))
    );
    assert_eq!(get(&dom, &styles, "c", "color"), Some(&rgb(255, 0, 0)));
    assert_eq!(get(&dom, &styles, "c", "font-size"), Some(&pt(10.0)));
    // An inherited property comes from the parent; a property that does not inherit does not.
    assert_eq!(get(&dom, &styles, "d", "color"), Some(&rgb(255, 165, 0)));
    assert_eq!(get(&dom, &styles, "d", "font-size"), Some(&pt(20.0)));
    // `em` in `font-size` counts the parent's size, and anywhere else the element's own.
    assert_eq!(get(&dom, &styles, "e", "font-size"), Some(&pt(10.0)));
    assert_eq!(get(&dom, &styles, "e", "margin-top"), Some(&pt(20.0)));
    assert_eq!(get(&dom, &styles, "f", "font-size"), Some(&pt(10.0)));
    assert_eq!(
        get(&dom, &styles, "f", "line-height"),
        Some(&Value::Number(1.5))
    );
    assert_eq!(get(&dom, &styles, "f", "margin-top"), None);
}

#[test]
fn a_shorthand_and_its_longhands_cascade_in_source_order() {
    let html = r#"<style>
        p { margin-left: 5pt; margin: 1pt 2pt }
        div { margin: 1pt 2pt; margin-left: 5pt; border: 2pt solid red; border-color: blue }
    </style><p id="p">a</p><div id="d">b</div>"#;
    let (dom, styles) = cascade(html);
    assert!(styles.dropped.is_empty(), "{:?}", styles.dropped);
    assert_eq!(get(&dom, &styles, "p", "margin-left"), Some(&pt(2.0)));
    assert_eq!(get(&dom, &styles, "d", "margin-left"), Some(&pt(5.0)));
    assert_eq!(get(&dom, &styles, "d", "margin-top"), Some(&pt(1.0)));
    assert_eq!(get(&dom, &styles, "d", "border-top-width"), Some(&pt(2.0)));
    assert_eq!(
        get(&dom, &styles, "d", "border-top-color"),
        Some(&rgb(0, 0, 255))
    );
}

#[test]
fn var_and_calc_resolve() {
    let html = r#"<style>
        :root { --brand: #ff0000; --gap: 4pt; --wide: calc(var(--gap) * 2) }
        p {
            color: var(--brand);
            padding-left: calc(var(--gap) + 1pt);
            margin-top: var(--wide);
            width: calc(100% - 2 * 10pt);
            border-left-color: var(--missing, blue);
            height: calc((1in - 36pt) / 2);
        }
        span { --brand: green }
    </style><p id="p">a <span id="s" style="color: var(--brand)">b</span></p>"#;
    let (dom, styles) = cascade(html);
    assert!(styles.dropped.is_empty(), "{:?}", styles.dropped);
    assert_eq!(get(&dom, &styles, "p", "color"), Some(&rgb(255, 0, 0)));
    assert_eq!(get(&dom, &styles, "p", "padding-left"), Some(&pt(5.0)));
    assert_eq!(get(&dom, &styles, "p", "margin-top"), Some(&pt(8.0)));
    assert_eq!(
        get(&dom, &styles, "p", "width"),
        Some(&Value::Length(Length {
            pt: -20.0,
            percent: 100.0
        }))
    );
    assert_eq!(
        get(&dom, &styles, "p", "border-left-color"),
        Some(&rgb(0, 0, 255))
    );
    assert_eq!(get(&dom, &styles, "p", "height"), Some(&pt(18.0)));
    assert_eq!(get(&dom, &styles, "s", "color"), Some(&rgb(0, 128, 0)));
}

#[test]
fn a_var_that_does_not_resolve_is_reported_once() {
    let html = "<style>\np { color: var(--none) }\n</style><p>a</p><p>b</p>";
    let (_, styles) = cascade(html);
    assert_eq!(styles.dropped.len(), 1, "{:?}", styles.dropped);
    let item = &styles.dropped[0];
    assert_eq!(
        (item.property.as_str(), item.value.as_str()),
        ("color", "var(--none)")
    );
    assert_eq!(
        (item.selector.as_str(), item.line, item.column),
        ("p", 2, 5)
    );
}

#[test]
fn media_print_applies_and_screen_does_not() {
    let html = r#"<style>
        p { color: black }
        @media print { p { color: red } }
        @media screen { p { color: blue } }
        @media all { p { font-size: 9pt } }
        @media only print, screen { p { text-align: center } }
        @media not screen { p { font-style: italic } }
        @media screen and (min-width: 100px) { p { font-size: 30pt } }
    </style>
    <style media="screen">p { font-weight: bold }</style>
    <style media="print">p { text-indent: 1pt }</style>
    <p id="a">x</p>"#;
    let (dom, styles) = cascade(html);
    assert!(styles.dropped.is_empty(), "{:?}", styles.dropped);
    assert_eq!(get(&dom, &styles, "a", "color"), Some(&rgb(255, 0, 0)));
    assert_eq!(get(&dom, &styles, "a", "font-size"), Some(&pt(9.0)));
    assert_eq!(
        get(&dom, &styles, "a", "text-align"),
        Some(&Value::Keyword("center".into()))
    );
    assert_eq!(
        get(&dom, &styles, "a", "font-style"),
        Some(&Value::Keyword("italic".into()))
    );
    assert_eq!(get(&dom, &styles, "a", "font-weight"), None);
    assert_eq!(get(&dom, &styles, "a", "text-indent"), Some(&pt(1.0)));
}

fn subset() -> serde_json::Value {
    serde_json::from_str(include_str!("../subset.json")).expect("subset.json is JSON")
}

fn names(subset: &serde_json::Value, list: &str) -> Vec<String> {
    subset[list]
        .as_array()
        .unwrap_or_else(|| panic!("subset.json has a `{list}` list"))
        .iter()
        .map(|item| {
            let description = item["description"].as_str().unwrap_or("");
            assert!(!description.is_empty(), "{item} has a description");
            item["name"]
                .as_str()
                .expect("every item has a name")
                .to_owned()
        })
        .collect()
}

#[test]
fn every_property_in_the_subset_table_parses() {
    let subset = subset();
    let listed = names(&subset, "properties");
    let mut engine: Vec<String> = properties::names().map(str::to_owned).collect();
    let mut sorted = listed.clone();
    sorted.sort();
    engine.sort();
    assert_eq!(
        sorted, engine,
        "subset.json and the engine list the same properties"
    );

    for item in subset["properties"].as_array().unwrap() {
        let name = item["name"].as_str().unwrap();
        let example = item["example"]
            .as_str()
            .expect("every property has an example");
        if let Some(p) = properties::property(name) {
            assert_eq!(
                item["inherited"].as_bool(),
                Some(p.inherited),
                "subset.json and the engine agree whether {name} inherits"
            );
        }
        let html = format!("<style>#x {{ {name}: {example} }}</style><div id=\"x\"></div>");
        let (dom, styles) = cascade(&html);
        assert!(
            styles.dropped.is_empty(),
            "{name}: {example} is dropped: {:?}",
            styles.dropped
        );
        let style = styles.of(by_id(&dom, "x")).unwrap();
        let set = properties::longhands(name)
            .unwrap()
            .iter()
            .any(|longhand| style.get(longhand).is_some());
        assert!(set, "{name}: {example} sets a longhand");
    }

    let at_rules = names(&subset, "atRules");
    for name in css::MARGIN_BOXES {
        assert!(
            at_rules.contains(&format!("@{name}")),
            "subset.json lists @{name}"
        );
    }
    let face = subset["atRules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "@font-face")
        .expect("subset.json lists @font-face");
    let descriptors: Vec<&str> = face["descriptors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d.as_str())
        .collect();
    assert_eq!(descriptors, css::FONT_FACE_DESCRIPTORS);
    names(&subset, "functions");
    names(&subset, "selectors");
    names(&subset, "html");
}

#[test]
fn a_declaration_outside_the_subset_is_dropped_and_reported_with_its_position() {
    let html = "<html><head><style>\n\
                p { color: red;\n  box-shadow: 1px 1px black; }\n\
                .x { position: sticky; transform: rotateX(10deg) }\n\
                @keyframes spin { from { color: red } }\n\
                p:hover { color: blue }\n\
                </style></head><body><p class=\"x\" style=\"filter: blur(2px)\">a</p></body></html>";
    let (dom, styles) = cascade(html);
    let line7 = html.lines().nth(6).unwrap();
    let filter_column = line7.find("filter").unwrap() as u64 + 1;
    let got: Vec<(&str, &str, &str, u64, u64)> = styles
        .dropped
        .iter()
        .map(|d| {
            (
                d.property.as_str(),
                d.value.as_str(),
                d.selector.as_str(),
                d.line,
                d.column,
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("box-shadow", "1px 1px black", "p", 3, 3),
            ("position", "sticky", ".x", 4, 6),
            ("transform", "rotateX(10deg)", ".x", 4, 24),
            ("@keyframes", "spin", "", 5, 1),
            ("", "", "p:hover", 6, 1),
            ("filter", "blur(2px)", "<p style>", 7, filter_column),
        ]
    );
    // The rest of the rule still applies.
    let p = styles.of(by_id_class(&dom, "x")).unwrap();
    assert_eq!(p.get("color"), Some(&rgb(255, 0, 0)));
    assert_eq!(p.get("position"), None);

    let rendered = nvs_pdf::render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .expect("the document renders");
    assert_eq!(rendered.dropped, styles.dropped);
}

fn by_id_class(dom: &parse::Dom, class: &str) -> usize {
    dom.nodes
        .iter()
        .position(|node| node.attr("class") == Some(class))
        .unwrap()
}

#[test]
fn a_clean_document_reports_nothing_dropped() {
    let html = r#"<!doctype html>
    <html lang="en"><head><title>Invoice</title>
    <style>
        @page { size: A4; margin: 2cm 1.5cm; @bottom-center { content: counter(page) " / " counter(pages) } }
        @page :first { margin-top: 3cm }
        :root { --ink: #222; --rule: 0.5pt solid #999 }
        body { font: 10pt/1.4 "Liberation Sans", sans-serif; color: var(--ink) }
        h1 { font-size: 18pt; margin: 0 0 8pt; break-after: avoid }
        table { width: 100%; border-collapse: collapse }
        th, td { padding: 4pt 6pt; border-bottom: var(--rule); text-align: left }
        td.amount { text-align: right; font-feature-settings: "tnum" 1 }
        tr:nth-child(2n) td { background-color: rgba(0, 0, 0, 0.04) }
        .total { font-weight: bold; border-top: 1pt solid black }
        li::before { content: "– "; color: gray }
        .header { position: running(header) }
    </style></head>
    <body><div class="header">Shop</div><h1>Invoice</h1>
    <table><tr><th>Item</th><th>Price</th></tr><tr><td>Tea</td><td class="amount">4.00</td></tr>
    <tr class="total"><td>Total</td><td class="amount" style="color: black">4.00</td></tr></table>
    <ul><li>Paid</li></ul></body></html>"#;
    let (dom, styles) = cascade(html);
    assert!(styles.dropped.is_empty(), "{:?}", styles.dropped);
    assert_eq!(styles.pages.len(), 2);
    assert_eq!(styles.pages[0].boxes[0].0, "bottom-center");
    let body = dom
        .nodes
        .iter()
        .position(|n| n.element(parse::Space::Html) == Some("body"))
        .unwrap();
    assert_eq!(
        styles.of(body).unwrap().get("color"),
        Some(&rgb(0x22, 0x22, 0x22))
    );
    assert!(!styles.before.is_empty());
    let rendered = nvs_pdf::render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .expect("the document renders");
    assert!(rendered.dropped.is_empty());
}
