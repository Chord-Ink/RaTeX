// Guard downstream exhaustive matches and struct literals against API breaks.
use ratex_layout::layout_box::BoxContent;
fn inspect(c: &BoxContent) {
    match c {
        BoxContent::HBox(..) => (),
        BoxContent::VBox(..) => (),
        BoxContent::Glyph { .. } => (),
        BoxContent::GlyphRun { .. } => (),
        BoxContent::Rule { .. } => (),
        BoxContent::Kern => (),
        BoxContent::Fraction { .. } => (),
        BoxContent::SupSub { .. } => (),
        BoxContent::Radical { .. } => (),
        BoxContent::OpLimits { .. } => (),
        BoxContent::Accent { .. } => (),
        BoxContent::LeftRight { .. } => (),
        BoxContent::Array { .. } => (),
        BoxContent::SvgPath { .. } => (),
        BoxContent::Framed { .. } => (),
        BoxContent::RaiseBox { .. } => (),
        BoxContent::Scaled { .. } => (),
        BoxContent::Angl { .. } => (),
        BoxContent::Overline { .. } => (),
        BoxContent::Underline { .. } => (),
        BoxContent::ProofTree { .. } => (),
        BoxContent::Empty => (),
    }
}
#[test]
fn public_layout_can_be_constructed_and_exhaustively_matched() {
    let box_ = ratex_layout::LayoutBox {
        width: 0.0,
        height: 0.0,
        depth: 0.0,
        content: BoxContent::Empty,
        color: ratex_types::color::Color::BLACK,
    };
    inspect(&box_.content);
    ratex_layout::validate_layout(&box_).unwrap();
    let _: ratex_types::display_item::DisplayList = ratex_layout::to_display_list(&box_);
}
