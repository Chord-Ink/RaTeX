use std::{cell::RefCell, collections::HashMap, rc::Rc};

use ratex_font::{get_char_metrics, FontId};
use ratex_layout::{
    layout, layout_with_glyph_source, to_display_list, validate_layout, FallbackGlyphMetrics,
    FallbackGlyphSource, LayoutOptions,
};
use ratex_parser::parse;
use ratex_types::display_item::DisplayItem;

type Key = (char, i64, String);

fn key(ch: char, size: f64, font: &str) -> Key {
    (ch, (size * 1e6).round() as i64, font.to_string())
}

struct Recorder {
    asks: RefCell<Vec<Key>>,
    answer: fn(char, f64) -> Option<FallbackGlyphMetrics>,
}

impl Recorder {
    fn new(answer: fn(char, f64) -> Option<FallbackGlyphMetrics>) -> Rc<Self> {
        Rc::new(Self {
            asks: RefCell::new(Vec::new()),
            answer,
        })
    }
}

impl FallbackGlyphSource for Recorder {
    fn metrics(&self, ch: char, size: f64, font: FontId) -> Option<FallbackGlyphMetrics> {
        self.asks.borrow_mut().push(key(ch, size, font.as_str()));
        (self.answer)(ch, size)
    }
}

fn metrics(width: f64) -> FallbackGlyphMetrics {
    FallbackGlyphMetrics {
        width,
        height: 0.53,
        depth: 0.1,
    }
}

fn none(_: char, _: f64) -> Option<FallbackGlyphMetrics> {
    None
}

fn drawn(source: &str, recorder: &Rc<Recorder>) -> Vec<DisplayItem> {
    let ast = parse(source).unwrap();
    let tree = layout_with_glyph_source(&ast, &LayoutOptions::default(), recorder.clone());
    validate_layout(&tree).unwrap();
    to_display_list(&tree).items
}

fn paths_of(items: &[DisplayItem], chars: &[char]) -> Vec<Key> {
    items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::GlyphPath {
                scale,
                font,
                char_code,
                ..
            } => {
                let ch = char::from_u32(*char_code)?;
                chars.contains(&ch).then(|| key(ch, *scale, font))
            }
            _ => None,
        })
        .collect()
}

fn glyph_x(items: &[DisplayItem], ch: char) -> f64 {
    items
        .iter()
        .find_map(|item| match item {
            DisplayItem::GlyphPath { x, char_code, .. } if *char_code == ch as u32 => Some(*x),
            _ => None,
        })
        .unwrap()
}

fn total_width(source: &str, recorder: &Rc<Recorder>) -> f64 {
    let ast = parse(source).unwrap();
    layout_with_glyph_source(&ast, &LayoutOptions::default(), recorder.clone()).width
}

#[test]
fn asked_glyphs_match_the_drawn_face_and_size() {
    let corpus: &[(&str, &str)] = &[
        (r"\text{ш}", "ш"),
        (r"\text{ш}^{\text{ш}}", "ш"),
        (r"\text{ш}_{\text{ш}^{\text{ш}}}", "ш"),
        (r"\frac{\text{ш}}{\text{ш}}", "ш"),
        (r"\tfrac{\text{ш}}{\text{ш}}", "ш"),
        (r"\genfrac{(}{)}{0pt}{1}{\text{ш}}{\text{ш}}", "ш"),
        (r"\sqrt[\text{ш}]{\text{ш}}", "ш"),
        (r"\sum_{\text{ш}}^{\text{ш}}", "ш"),
        (r"\left(\text{ш}\right)", "ш"),
        (r"\scriptstyle\text{ш}", "ш"),
        (r"\Large\text{ш}", "ш"),
        (r"\tiny\text{ш}", "ш"),
        (r"\Huge\text{ш}^{\text{ш}}", "ш"),
        (r"\Huge{\tiny\text{ш}}", "ш"),
        (r"\htmlStyle{font-size:2em}{\text{ш}}", "ш"),
        (r"\text{ш}^{\htmlStyle{font-size:2em}{\text{ш}}}", "ш"),
        (r"\htmlStyle{font-size:2em}{\text{ш}^{\text{ш}}}", "ш"),
        (
            r"\htmlStyle{font-size:2em}{\htmlStyle{font-size:0.5em}{\text{ш}}}",
            "ш",
        ),
        (
            r"\begin{CD}A @>\text{ш}>> B\\ @V\text{ш}VV @.\\ C @= D\end{CD}",
            "ш",
        ),
        (r"\verb|ш|", "ш"),
        (r"\operatorname{ш}x", "ш"),
        (r"\text{∁}", "∁"),
        (r"\text{😀}", "😀"),
        (r"\text{日本}x", "日本"),
        (r"日^{日}", "日"),
        (r"𝓐x^{𝓐}", "𝓐"),
        (r"𝓐\mathbf{𝓐}", "𝓐"),
        (r"\mathrm{ш}", "ш"),
    ];
    for (source, chars) in corpus {
        let chars: Vec<char> = chars.chars().collect();
        let recorder = Recorder::new(none);
        let items = drawn(source, &recorder);
        let paths = paths_of(&items, &chars);
        assert!(!paths.is_empty(), "{source}: nothing drawn");
        let asks: Vec<Key> = recorder
            .asks
            .borrow()
            .iter()
            .filter(|ask| chars.contains(&ask.0))
            .cloned()
            .collect();
        for path in &paths {
            assert!(
                asks.contains(path),
                "{source}: drawn {path:?} not asked, asked {asks:?}"
            );
        }
        for ask in &asks {
            assert!(
                paths.contains(ask),
                "{source}: asked {ask:?} not drawn, drawn {paths:?}"
            );
        }
    }
}

#[test]
fn phantoms_are_asked_and_not_drawn() {
    for source in [
        r"\phantom{\text{ш}}x",
        r"\hphantom{\text{ш}}x",
        r"\vphantom{\text{ш}}x",
    ] {
        let recorder = Recorder::new(none);
        let items = drawn(source, &recorder);
        assert!(paths_of(&items, &['ш']).is_empty(), "{source}");
        assert!(
            recorder.asks.borrow().iter().any(|ask| ask.0 == 'ш'),
            "{source}"
        );
    }
}

#[test]
fn the_wide_switch_names_cjk_before_the_lookup() {
    let recorder = Recorder::new(none);
    drawn(r"\text{日}\text{ш}", &recorder);
    let asks = recorder.asks.borrow();
    assert!(asks.contains(&key('日', 1.0, "CJK-Regular")), "{asks:?}");
    assert!(!asks.contains(&key('ш', 1.0, "CJK-Regular")), "{asks:?}");
}

#[test]
fn each_size_gets_its_own_width() {
    fn per_size(ch: char, size: f64) -> Option<FallbackGlyphMetrics> {
        (ch == 'ш').then(|| metrics(if size > 0.9 { 0.8 } else { 0.9 }))
    }
    fn zero(ch: char, _: f64) -> Option<FallbackGlyphMetrics> {
        (ch == 'ш').then(|| metrics(0.0))
    }
    let source = r"\text{ш}^{\text{ш}}";
    let with = total_width(source, &Recorder::new(per_size));
    let without = total_width(source, &Recorder::new(zero));
    assert!(
        (with - without - (0.8 + 0.9 * 0.7)).abs() < 1e-9,
        "{with} {without}"
    );
}

#[test]
fn operator_names_take_the_source_width() {
    fn wide(ch: char, _: f64) -> Option<FallbackGlyphMetrics> {
        (ch == 'ш').then(|| metrics(0.8))
    }
    fn narrow(ch: char, _: f64) -> Option<FallbackGlyphMetrics> {
        (ch == 'ш').then(|| metrics(0.3))
    }
    let source = r"\operatorname{ш}x";
    let wide = glyph_x(&drawn(source, &Recorder::new(wide)), 'x');
    let narrow = glyph_x(&drawn(source, &Recorder::new(narrow)), 'x');
    assert!((wide - narrow - 0.5).abs() < 1e-9, "{wide} {narrow}");
}

#[test]
fn an_empty_source_matches_layout() {
    for source in [
        r"\text{ш}^{\text{ш}}",
        r"\operatorname{ш}x",
        r"\verb|ш|",
        r"\text{∁日😀}",
        r"𝓐x\htmlStyle{font-size:2em}{\text{ш}}",
        r"\frac{\text{ш}}{\sqrt{y}}",
    ] {
        let ast = parse(source).unwrap();
        let options = LayoutOptions::default();
        let with = layout_with_glyph_source(&ast, &options, Rc::new(HashMap::new()));
        assert_eq!(
            serde_json::to_value(to_display_list(&with)).unwrap(),
            serde_json::to_value(to_display_list(&layout(&ast, &options))).unwrap(),
            "{source}"
        );
    }
}

#[test]
fn katex_glyphs_are_never_asked() {
    assert!(get_char_metrics(FontId::MainRegular, 'x' as u32).is_some());
    let recorder = Recorder::new(none);
    drawn(
        r"x^2 + \frac{a}{b} + \sum_i \text{abc} + \operatorname{sin}x",
        &recorder,
    );
    assert!(
        recorder.asks.borrow().is_empty(),
        "{:?}",
        recorder.asks.borrow()
    );
}
