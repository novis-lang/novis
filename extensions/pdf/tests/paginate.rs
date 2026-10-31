//! Pagination on the host: the `@page` rules, forced breaks, margin boxes, links, bookmarks
//! and metadata (`rule:core-classes/pdf-css-subset-table`).

use nvs_pdf::layout::{self, Bookmark, Href, Item, Laid};
use nvs_pdf::write::Info;
use nvs_pdf::{Document, assets, css, fonts, parse, render};

fn lay(html: &str) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(&[]);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}

/// Each text run on `page`: its text, its pen start and its baseline.
fn texts(laid: &Laid, page: usize) -> Vec<(String, f32, f32)> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, x, y, .. } => Some((text.trim().to_owned(), *x, *y)),
            _ => None,
        })
        .filter(|(text, ..)| !text.is_empty())
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn page_size_margins_and_first_left_right_blank() {
    let laid = lay("<style>
        @page { size: A5 landscape; margin: 20mm }
        @page :first { margin-top: 40mm }
        @page :left { margin-left: 30mm; margin-right: 10mm }
        @page :right { margin-left: 10mm; margin-right: 30mm }
        @page :blank { size: A4 }
        body, p { margin: 0 }
        </style>
        <p>Shop</p><p style='break-before: page'>Blog</p><p style='break-before: left'>News</p>");
    assert_eq!(laid.pages.len(), 4);
    let [first, left, blank, last] = [0, 1, 2, 3].map(|at| laid.pages[at].geometry);

    // The first page is a right page, A5 turned on its side, with the `:first` top margin.
    assert!(
        close(first.width, mm(210.0)) && close(first.height, mm(148.0)),
        "{first:?}"
    );
    let want = [mm(40.0), mm(30.0), mm(20.0), mm(10.0)];
    assert!(
        first.margin.iter().zip(want).all(|(a, b)| close(*a, b)),
        "{first:?}"
    );
    let [(text, x, y)] = &texts(&laid, 0)[..] else {
        panic!("{:?}", texts(&laid, 0));
    };
    assert_eq!(text, "Shop");
    assert!(
        close(*x, mm(10.0)) && *y > mm(40.0) && *y < mm(40.0) + 20.0,
        "{x} {y}"
    );

    // The second page is a left one, with the plain top margin.
    assert!(
        close(left.margin[3], mm(30.0)) && close(left.margin[0], mm(20.0)),
        "{left:?}"
    );
    let [(text, x, _)] = &texts(&laid, 1)[..] else {
        panic!("{:?}", texts(&laid, 1));
    };
    assert_eq!(text, "Blog");
    assert!(close(*x, mm(30.0)), "{x}");

    // A break to a left page after a left page leaves a blank right page between them.
    assert!(laid.pages[2].blank && laid.pages[2].items.is_empty());
    assert!(close(blank.width, layout::PAGE_WIDTH) && close(blank.height, layout::PAGE_HEIGHT));
    assert!(!last.margin.is_empty() && !laid.pages[3].blank);
    assert_eq!(texts(&laid, 3)[0].0, "News");
    assert!(close(texts(&laid, 3)[0].1, mm(30.0)));

    // A custom size takes its width first, and a margin's percentage counts against the side.
    let custom = lay("<style>@page { size: 100mm 50mm; margin: 10% }</style><p>Shop</p>");
    let page = custom.pages[0].geometry;
    assert!(
        close(page.width, mm(100.0)) && close(page.height, mm(50.0)),
        "{page:?}"
    );
    assert!(
        close(page.margin[0], mm(5.0)) && close(page.margin[3], mm(10.0)),
        "{page:?}"
    );

    // A size that does not read is A4, and the file's pages carry their own sizes.
    let unread = lay("<style>@page { size: A4 tall }</style><p>Shop</p>");
    assert!(close(unread.pages[0].geometry.height, layout::PAGE_HEIGHT));
    let rendered = render(&Document {
        html: "<style>@page { size: 300pt 200pt }</style><p>Shop</p>".to_owned(),
        ..Document::default()
    })
    .unwrap();
    assert!(contains(&rendered.data, b"/MediaBox[0 0 300 200]"));
}

/// The text runs on `page` whose baseline is between `top` and `bottom`, joined in order, and
/// the left and right end of the first and last.
fn band(laid: &Laid, page: usize, top: f32, bottom: f32) -> (String, f32, f32) {
    let mut text = String::new();
    let (mut left, mut right) = (f32::MAX, f32::MIN);
    for item in &laid.pages[page].items {
        if let Item::Text {
            text: run,
            x,
            y,
            glyphs,
            ..
        } = item
            && *y > top
            && *y < bottom
        {
            text.push_str(run);
            left = left.min(*x);
            let inked = glyphs
                .iter()
                .rev()
                .skip_while(|g| run[g.text.clone()].trim().is_empty());
            right = right.max(x + inked.map(|g| g.advance).sum::<f32>());
        }
    }
    (text.trim().to_owned(), left, right)
}

#[test]
fn margin_boxes_print_page_x_of_y() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 50pt;
            @bottom-right { content: 'Page ' counter(page) ' of ' counter(pages) }
            @top-center { content: 'Shop'; font-size: 10pt }
            @right-middle { content: counter(page, upper-roman) }
        }
        @page :first { @bottom-right { content: none } }
        body, p { margin: 0 }
        p { break-after: page }
        </style><p>One</p><p>Two</p><p>Three</p>");
    assert_eq!(laid.pages.len(), 3);
    // The first page's own rule removes its footer.
    assert_eq!(band(&laid, 0, 250.0, 300.0).0, "");
    for page in 1..3 {
        let (footer, _, right) = band(&laid, page, 250.0, 300.0);
        assert_eq!(footer, format!("Page {} of 3", page + 1));
        // The bottom-right box is the whole side when it is alone there, and its text ends at
        // the content box's right edge.
        assert!(close(right, 350.0), "{right}");
    }
    for page in 0..3 {
        // The top box is alone on its side, so its text is centred on the page.
        let (header, left, right) = band(&laid, page, 0.0, 50.0);
        assert_eq!(header, "Shop");
        assert!(((left + right) / 2.0 - 200.0).abs() < 0.5, "{left} {right}");
        // The right-side box is centred in the right margin, half way down.
        let roman = ["I", "II", "III"][page];
        let side = laid.pages[page].items.iter().find_map(|item| match item {
            Item::Text { text, x, y, .. } if text.trim() == roman && *x > 350.0 => Some(*y),
            _ => None,
        });
        assert!(side.is_some_and(|y| y > 120.0 && y < 180.0), "{side:?}");
    }
}

#[test]
fn a_running_element_repeats_on_every_page() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 50pt;
            @top-center { content: element(head) }
            @bottom-left { content: element(foot, last) }
        }
        body, p, div { margin: 0 }
        .head { position: running(head); font-size: 10pt }
        .foot { position: running(foot) }
        p { break-after: page }
        </style>
        <div class=head>Shop</div><p>One</p><p>Two</p>
        <div class=head>News</div><div class=foot>Blog</div><div class=foot>Team</div>
        <p>Three</p>");
    assert_eq!(laid.pages.len(), 3);
    // A running element takes no room, so the body starts at the top of the content box.
    let (body, ..) = band(&laid, 0, 50.0, 250.0);
    assert_eq!(body, "One");
    let one = texts(&laid, 0)
        .into_iter()
        .find(|(t, ..)| t == "One")
        .unwrap();
    assert!(one.2 > 50.0 && one.2 < 50.0 + 20.0, "{one:?}");
    for (page, want) in [(0, "Shop"), (1, "Shop"), (2, "News")] {
        // The page's first running element, or the last of an earlier page, is the header,
        // centred on the page because its box is alone on the top side.
        let (header, left, right) = band(&laid, page, 0.0, 50.0);
        assert_eq!(header, want, "page {page}");
        assert!(((left + right) / 2.0 - 200.0).abs() < 0.5, "{left} {right}");
    }
    // `last` takes the last element on its page, and no page before the first has the box.
    assert_eq!(band(&laid, 0, 250.0, 300.0).0, "");
    assert_eq!(band(&laid, 1, 250.0, 300.0).0, "");
    let (footer, left, _) = band(&laid, 2, 250.0, 300.0);
    assert_eq!(footer, "Team");
    assert!(close(left, 50.0), "{left}");
}

/// The text of each page, one entry per line.
fn lines(laid: &Laid) -> Vec<Vec<String>> {
    (0..laid.pages.len())
        .map(|page| texts(laid, page).into_iter().map(|(t, ..)| t).collect())
        .collect()
}

/// A page 200pt tall inside its margins, with lines 20pt tall, so ten lines fill it.
const TEN_LINES: &str = "<style>
    @page { size: 400pt 300pt; margin: 50pt }
    body, p, div { margin: 0; line-height: 20pt }
    </style>";

#[test]
fn break_before_after_and_inside_avoid_are_kept() {
    let filler = "<p>Line</p>".repeat(8);
    let tall = "Tall<br>".repeat(12);
    let laid = lay(&format!(
        "{TEN_LINES}{filler}
        <div style='break-inside: avoid'>Shop<br>Blog<br>News</div>
        <p style='page-break-before: always'>Team</p>
        <div style='page-break-inside: avoid'>{tall}</div>
        <p style='page-break-after: always'>Last</p><p>After</p>"
    ));
    let pages = lines(&laid);
    // Three lines do not fit under eight, so the box starts on the next page whole.
    assert_eq!(pages[0], vec!["Line"; 8], "{pages:?}");
    assert_eq!(pages[1], ["Shop", "Blog", "News"], "{pages:?}");
    let shop = texts(&laid, 1)[0].2;
    assert!(shop > 50.0 && shop < 70.0, "{shop}");
    // `page-break-before` starts a page, and a box taller than a page is split where it is.
    assert_eq!(pages[2][0], "Team", "{pages:?}");
    assert_eq!(pages[2][1..], vec!["Tall"; 9], "{pages:?}");
    assert_eq!(
        pages[3],
        [vec!["Tall"; 3], vec!["Last"]].concat(),
        "{pages:?}"
    );
    // `page-break-after` puts what follows on the next page.
    assert_eq!(pages[4], ["After"], "{pages:?}");
    assert_eq!(pages.len(), 5);

    // A box that fits under the last block stays there.
    let laid = lay(&format!(
        "{TEN_LINES}{}<div style='break-inside: avoid'>Shop<br>Blog</div>",
        "<p>Line</p>".repeat(8)
    ));
    assert_eq!(lines(&laid).len(), 1);
}

#[test]
fn orphans_and_widows_hold_lines_together() {
    let paragraph = "<br>Shop".repeat(3);
    // Two of four lines would go to the next page, under `widows: 3`, so only one stays.
    let laid = lay(&format!(
        "{TEN_LINES}{}<p style='orphans: 1; widows: 3'>Shop{paragraph}</p>",
        "<p>Line</p>".repeat(8)
    ));
    let pages = lines(&laid);
    assert_eq!(pages[0].len(), 9, "{pages:?}");
    assert_eq!(pages[1], ["Shop"; 3], "{pages:?}");

    // One line would stay at the bottom under `orphans: 2`, so the paragraph moves whole.
    let laid = lay(&format!(
        "{TEN_LINES}{}<p>Blog<br>Blog<br>Blog</p>",
        "<p>Line</p>".repeat(9)
    ));
    let pages = lines(&laid);
    assert_eq!(pages[0], vec!["Line"; 9], "{pages:?}");
    assert_eq!(pages[1], ["Blog"; 3], "{pages:?}");

    // The default of two lines each way moves one line down with its neighbour.
    let laid = lay(&format!(
        "{TEN_LINES}{}<p>News<br>News<br>News<br>News</p>",
        "<p>Line</p>".repeat(7)
    ));
    let pages = lines(&laid);
    assert_eq!(pages[0].len(), 9, "{pages:?}");
    assert_eq!(pages[1], ["News"; 2], "{pages:?}");

    // A paragraph that cannot keep both fills its page and breaks where the page ends.
    let laid = lay(&format!(
        "{TEN_LINES}<p style='orphans: 9; widows: 9'>{}Team</p>",
        "Team<br>".repeat(11)
    ));
    let pages = lines(&laid);
    assert_eq!((pages[0].len(), pages[1].len()), (10, 2), "{pages:?}");
}

/// The text runs on `page` whose baseline is between `top` and `bottom` and that start left of
/// `split`, or at it and right of it when `right`, joined in order.
fn corner(laid: &Laid, page: usize, (top, bottom): (f32, f32), split: f32, right: bool) -> String {
    texts(laid, page)
        .into_iter()
        .filter(|(_, x, y)| *y > top && *y < bottom && (*x >= split) == right)
        .map(|(t, ..)| t)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn string_set_carries_the_chapter_title() {
    let filler = "<p>Line</p>".repeat(12);
    let laid = lay(&format!(
        "{TEN_LINES}<style>
        @page {{
            @top-left {{ content: 'Chapter: ' string(chapter) }}
            @top-right {{ content: string(Chapter, last) }}
            @bottom-left {{ content: string(chapter, start) }}
            @bottom-right {{ content: string(chapter, first-except) }}
        }}
        h1 {{ string-set: chapter content(text); break-before: page; margin: 0; font-size: 12pt }}
        .part {{ string-set: chapter 'Shop ' attr(data-part) }}
        </style>
        <p>Intro</p>
        <h1>Shop   Prices</h1><p class=part data-part=two>Team</p>{filler}
        <h1>Blog</h1>"
    ));
    assert_eq!(laid.pages.len(), 4);
    let head = |page, right| corner(&laid, page, (0.0, 50.0), 200.0, right);
    let foot = |page, right| corner(&laid, page, (250.0, 300.0), 200.0, right);

    // Before the first `string-set` the string is empty.
    assert_eq!(
        (head(0, false), head(0, true)),
        ("Chapter:".to_owned(), String::new())
    );
    // The first value set on the page, with its spaces collapsed, and the last one.
    assert_eq!(head(1, false), "Chapter: Shop Prices");
    assert_eq!(head(1, true), "Shop two");
    // `start` takes the value set at the top of the page, and `first-except` is empty there.
    assert_eq!(foot(1, false), "Shop Prices");
    assert_eq!(foot(1, true), "");
    // A page that sets nothing has the last value of an earlier page, in every box.
    assert_eq!(head(2, false), "Chapter: Shop two");
    assert_eq!(
        (foot(2, false), foot(2, true)),
        ("Shop two".to_owned(), "Shop two".to_owned())
    );
    assert_eq!(head(3, false), "Chapter: Blog");
}

#[test]
fn named_pages_switch_page_size() {
    let words = "Shop ".repeat(80);
    let laid = lay(&format!(
        "{TEN_LINES}<style>
        @page wide {{ size: 600pt 300pt; @top-center {{ content: 'Wide' }} }}
        .wide {{ page: wide }}
        </style>
        <p>Intro</p><div class=wide><p>{words}</p>{}</div><p>News</p>",
        "<p>Blog</p>".repeat(12)
    ));
    let names: Vec<Option<&str>> = laid.pages.iter().map(|p| p.name.as_deref()).collect();
    // A box with another page name starts a page, and a page it goes on to keeps the name.
    assert_eq!(names, [None, Some("wide"), Some("wide"), None]);
    for (page, width) in [(0, 400.0), (1, 600.0), (2, 600.0), (3, 400.0)] {
        assert!(close(laid.pages[page].geometry.width, width), "page {page}");
        let header = band(&laid, page, 0.0, 50.0).0;
        assert_eq!(
            header,
            if width > 400.0 { "Wide" } else { "" },
            "page {page}"
        );
    }
    assert_eq!(lines(&laid)[3], ["News"]);
    // The lines on a wide page are as wide as its content box.
    let (_, left, right) = band(&laid, 1, 50.0, 70.0);
    assert!(
        close(left, 50.0) && right > 500.0 && right <= 550.05,
        "{left} {right}"
    );

    // A first box with a page name names the first page, with no page before it.
    let laid = lay(&format!(
        "{TEN_LINES}<style>@page wide {{ size: 600pt 300pt }}</style>
        <div style='page: wide'>Shop</div>"
    ));
    assert_eq!(laid.pages.len(), 1);
    assert!(close(laid.pages[0].geometry.width, 600.0));
}

/// What each text run on `page` with its baseline between `top` and `bottom` draws, and its
/// left and right end, left to right.
fn drawn(laid: &Laid, page: usize, top: f32, bottom: f32) -> Vec<(String, f32, f32)> {
    let mut out: Vec<(String, f32, f32)> = laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                text, x, y, glyphs, ..
            } if *y > top && *y < bottom => {
                let mut drawn = String::new();
                for g in glyphs {
                    drawn.push_str(&text[g.text.clone()]);
                }
                let advance: f32 = glyphs.iter().map(|g| g.advance).sum();
                Some((drawn.trim().to_owned(), *x, x + advance))
            }
            _ => None,
        })
        .filter(|(text, ..)| !text.is_empty())
        .collect();
    out.sort_by(|a, b| a.1.total_cmp(&b.1));
    out
}

#[test]
fn target_counter_and_leader_build_a_table_of_contents() {
    let laid = lay(&format!(
        "{TEN_LINES}<style>
        .toc a::after {{ content: leader('.') target-counter(attr(href), page) }}
        .toc a.roman::after {{ content: leader(dotted) target-counter(attr(href), page, upper-roman) }}
        h2 {{ break-before: page; margin: 0; font-size: 12pt }}
        </style>
        <p class=toc><a href='#shop'>Shop</a></p>
        <p class=toc><a href='#blog'>Blog</a></p>
        <p class=toc><a class=roman href='#news'>News</a></p>
        <p class=toc><a href='#gone'>Gone</a></p>
        <h2 id=shop>Shop</h2><h2 id=blog>Blog</h2>{}<h2 id=news>News</h2>",
        "<p>Line</p>".repeat(12)
    ));
    assert_eq!(laid.pages.len(), 5);
    // Each entry is its title, a run of dots, and the page its heading is on, which ends at
    // the content box's right edge.
    for (line, title, page) in [(0, "Shop", "2"), (1, "Blog", "3"), (2, "News", "V")] {
        let top = 50.0 + 20.0 * line as f32;
        let runs = drawn(&laid, 0, top, top + 20.0);
        let [(first, ..), (dots, start, end), (number, after, right)] = &runs[..] else {
            panic!("{runs:?}");
        };
        assert_eq!((first.as_str(), number.as_str()), (title, page));
        assert!(dots.len() > 20 && dots.chars().all(|c| c == '.'), "{dots}");
        assert!(*start > 70.0 && *end <= after + 0.01, "{runs:?}");
        assert!(close(*right, 350.0), "{right}");
    }
    // An entry whose heading is not in the document has no number, and its dots fill the line.
    let runs = drawn(&laid, 0, 110.0, 130.0);
    let [(first, ..), (dots, _, end)] = &runs[..] else {
        panic!("{runs:?}");
    };
    assert_eq!(first, "Gone");
    assert!(dots.chars().all(|c| c == '.') && *end > 340.0, "{runs:?}");
}

#[test]
fn page_numbers_that_never_settle_throw() {
    // `III` takes three lines and puts the heading on the fourth page, and `IV` takes two and
    // puts it on the third, so the page number never stops changing.
    let html = format!(
        "{TEN_LINES}<style>
        .n {{ width: 15pt; font-size: 20pt; font-family: monospace; word-break: break-all }}
        .n::before {{ content: target-counter('#x', page, upper-roman) }}
        </style>
        <p>Shop</p><p style='break-before: page'>Blog</p><p style='break-before: page'>Blog</p>
        {}<div class=n></div><p id=x>News</p>",
        "<p>Line</p>".repeat(6)
    );
    let error = render(&Document {
        html,
        ..Document::default()
    })
    .unwrap_err();
    let nvs_pdf::Error::Runtime(message) = error else {
        panic!("{error:?}");
    };
    assert!(message.contains("5 layout passes"), "{message}");

    // A number that settles on the second pass lays the document out once more and no more.
    let laid = lay(&format!(
        "{TEN_LINES}<style>.n::before {{ content: target-counter('#x', page) }}</style>
        <p class=n></p><p id=x style='break-before: page'>News</p>"
    ));
    assert_eq!(drawn(&laid, 0, 50.0, 70.0)[0].0, "2");
}

fn rendered(html: &str) -> Vec<u8> {
    render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .unwrap()
    .data
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

#[test]
fn headings_become_bookmarks() {
    let html = format!(
        "{TEN_LINES}<style>h1, h2, h3 {{ margin: 0; font-size: 12pt; line-height: 20pt }}</style>
        <h1>Shop</h1><h2>Blog
        one</h2>{}<h2>News</h2><h3>Team</h3><h1>Help</h1><h2></h2>",
        "<p>Line</p>".repeat(12)
    );
    // Each heading's page index and top: ten lines fit on a page, so the last three are on the
    // second page under the four lines left over. An empty heading has no entry.
    let laid = lay(&html);
    let want = [
        (1, "Shop", 0, 50.0),
        (2, "Blog one", 0, 70.0),
        (2, "News", 1, 130.0),
        (3, "Team", 1, 150.0),
        (1, "Help", 1, 170.0),
    ];
    assert_eq!(laid.bookmarks.len(), want.len(), "{:?}", laid.bookmarks);
    for (
        Bookmark {
            level,
            title,
            page,
            y,
        },
        (l, t, p, top),
    ) in laid.bookmarks.iter().zip(want)
    {
        assert_eq!((*level, title.as_str(), *page), (l, t, p));
        assert!(close(*y, top), "{title} at {y}");
    }

    // The outline has the five entries, `Blog` and `News` under `Shop` and `Team` under `News`.
    let data = rendered(&html);
    assert_eq!(count(&data, b"/Type/Outlines"), 1);
    for title in ["Shop", "Blog one", "News", "Team", "Help"] {
        assert!(
            contains(&data, format!("/Title({title})").as_bytes()),
            "{title}"
        );
    }
    // A closed entry's `/Count` is minus the number of entries directly under it.
    assert!(contains(&data, b"/Count -2/Title(Shop)"));
    assert!(contains(&data, b"/Count -1/Title(News)"));
    assert!(contains(&data, b"/Count 2>>"));
    assert!(!contains(&rendered("<p>Shop</p>"), b"/Outlines"));
}

#[test]
fn internal_and_external_links_are_annotations() {
    let html = format!(
        "{TEN_LINES}<p><a href='https://example.com/shop'>Shop</a> and
        <a href=' MAILTO:team@example.com'>mail <b>us</b></a> and <a href='#news'>News</a>
        <a href='javascript:alert(1)'>Bad</a> <a href='file:///etc/passwd'>Disk</a>
        <a href='#gone'>Gone</a> <a>Plain</a></p>{}<p id=news>News</p>",
        "<p>Line</p>".repeat(9)
    );
    let laid = lay(&html);
    let links: Vec<(Href, f32, f32, f32, f32)> = laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Link {
                href,
                x,
                y,
                width,
                height,
            } => Some((href.clone(), *x, *y, *width, *height)),
            _ => None,
        })
        .collect();
    // `mail us` is two runs in two styles, so it is two boxes. A `javascript:` or `file:` link,
    // and an `<a>` with no `href`, is no link.
    let hrefs: Vec<&Href> = links.iter().map(|l| &l.0).collect();
    let uri = |s: &str| Href::Uri(s.to_owned());
    let anchor = |s: &str| Href::Anchor(s.to_owned());
    assert_eq!(
        hrefs,
        [
            &uri("https://example.com/shop"),
            &uri("MAILTO:team@example.com"),
            &uri("MAILTO:team@example.com"),
            &anchor("news"),
            &anchor("gone"),
        ]
    );
    // The first box covers the text `Shop` from its ascent to its descent.
    let (_, x, y, width, height) = links[0];
    let (text, start, end) = &drawn(&laid, 0, 50.0, 70.0)[0];
    assert!(text.starts_with("Shop"), "{text}");
    assert!(close(x, *start) && x + width <= *end + 0.01, "{links:?}");
    assert!(
        (50.0..60.0).contains(&y) && height > 8.0 && y + height <= 72.0,
        "{links:?}"
    );
    // The paragraph with `id=news` is the first line of the second page.
    let (page, top) = laid.anchors["news"];
    assert!(page == 1 && close(top, 50.0), "{page} {top}");

    // The file has a URI action for each address and a destination for `#news`, and the link
    // to `#gone` is left out because no element has that `id`.
    let data = rendered(&html);
    assert_eq!(count(&data, b"/Subtype/Link"), 4);
    assert!(contains(&data, b"/URI(https://example.com/shop)"));
    assert_eq!(count(&data, b"/URI(MAILTO:team@example.com)"), 2);
    for word in [&b"javascript"[..], b"/JavaScript", b"passwd", b"/Launch"] {
        assert!(!contains(&data, word), "{}", String::from_utf8_lossy(word));
    }
}

#[test]
fn title_meta_and_lang_become_metadata() {
    let html = "<!doctype html><html lang=de><head><title> Shop
        invoice </title><title>Second</title>
        <meta name=author content='Shop team'><meta name=Description content='An invoice'>
        <meta name=keywords content='invoice, shop,'><meta name=generator content=Blog>
        </head><body><p>Shop</p></body></html>";
    let info = Info::of(&parse::parse(html));
    assert_eq!(
        info,
        Info {
            title: Some("Shop invoice".to_owned()),
            authors: vec!["Shop team".to_owned()],
            description: Some("An invoice".to_owned()),
            keywords: vec!["invoice".to_owned(), "shop".to_owned()],
            lang: Some("de".to_owned()),
        }
    );
    let data = rendered(html);
    for entry in [
        &b"/Title(Shop invoice)"[..],
        b"/Author(Shop team)",
        b"/Subject(An invoice)",
        b"/Lang(de)",
    ] {
        assert!(contains(&data, entry), "{}", String::from_utf8_lossy(entry));
    }
    assert!(contains(&data, b"invoice") && !contains(&data, b"Second"));
    // A document with none of them has no title and no language. The tag tree's role map
    // names a `/Title` role, so the entries are matched with their value.
    let data = rendered("<p>Shop</p>");
    assert!(!contains(&data, b"/Title(") && !contains(&data, b"/Lang("));
}

/// Twelve lines, so what comes after them starts on the second page.
fn twelve() -> String {
    "<p>Line</p>".repeat(12)
}

/// The value the page at `page` gives a named string or a mark of `name`.
fn fact<'l>(facts: &'l [(String, String)], name: &str) -> Option<&'l str> {
    facts
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

#[test]
fn a_mark_or_string_in_a_table_cell_is_on_the_page_its_row_is_drawn_on() {
    let laid = lay(&format!(
        "{TEN_LINES}<style>td {{ padding: 0 }} .s {{ string-set: cell content(text) }}</style>
        {}<table><tr><td><div data-mark='row: two'>Shop</div><div class=s>Blog</div></td></tr>
        </table>",
        twelve()
    ));
    assert_eq!(laid.pages.len(), 2);
    assert_eq!(fact(&laid.pages[0].marks, "row"), None);
    assert_eq!(fact(&laid.pages[0].strings, "cell"), None);
    assert_eq!(fact(&laid.pages[1].marks, "row"), Some("two"));
    assert_eq!(fact(&laid.pages[1].strings, "cell"), Some("Blog"));
}

#[test]
fn a_string_or_running_element_in_an_inline_block_float_or_flex_item_is_on_its_page() {
    let laid = lay(&format!(
        "{TEN_LINES}<style>
        @page {{ @top-center {{ content: element(head) }} }}
        .a {{ string-set: a content(text) }}
        .b {{ string-set: b content(text) }}
        .c {{ string-set: c content(text) }}
        .head {{ position: running(head) }}
        </style>
        {}<div style='display: inline-block'><div class=a>Shop</div>
        <div class=head>Team</div></div>
        <div style='float: left'><div class=b>Blog</div></div>
        <div style='display: flex'><div><div class=c>News</div></div></div>",
        twelve()
    ));
    assert_eq!(laid.pages.len(), 2);
    for (name, value) in [("a", "Shop"), ("b", "Blog"), ("c", "News")] {
        assert_eq!(fact(&laid.pages[0].strings, name), None, "{name}");
        assert_eq!(fact(&laid.pages[1].strings, name), Some(value), "{name}");
    }
    // The page before the running element's has no header, and its own page draws it.
    assert_eq!(band(&laid, 0, 0.0, 50.0).0, "");
    assert_eq!(band(&laid, 1, 0.0, 50.0).0, "Team");
}

#[test]
fn an_id_in_an_inline_block_float_flex_item_cell_or_running_element_has_its_page() {
    let laid = lay(&format!(
        "{TEN_LINES}<style>.head {{ position: running(head) }}</style>
        {}<div style='display: inline-block'><p id=shop>Shop</p></div>
        <div style='float: left'><p id=blog>Blog</p></div>
        <div style='display: flex'><div><p id=news>News</p></div></div>
        <table><tr><td><p id=team>Team</p></td></tr></table>
        <div class=head><p id=help>Help</p></div><p>End</p>",
        twelve()
    ));
    assert_eq!(laid.pages.len(), 2);
    for id in ["shop", "blog", "news", "team", "help"] {
        let (page, top) = laid.anchors[id];
        assert_eq!(page, 1, "{id}");
        assert!(top >= 50.0 && top < 250.0, "{id} at {top}");
    }
}
