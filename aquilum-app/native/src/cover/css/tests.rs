use super::calc::value;
use super::color::parse_color;
use super::stylesheet::parse_patterns;
use super::types::Kind;

#[test]
fn calc_values_and_colors() {
    assert_eq!(value("calc(250% / 3)", 16.0).unwrap().pct, 250.0 / 3.0);
    assert_eq!(value("calc(-46px / 4)", 16.0).unwrap().px, -11.5);
    assert_eq!(value("calc(3 * 46px / 4)", 16.0).unwrap().px, 34.5);
    assert!((value("calc(25% / -6)", 16.0).unwrap().pct + 25.0 / 6.0).abs() < 1e-9);
    assert_eq!(value("1.2em", 8.0).unwrap().px, 9.6);
    assert_eq!(value("-120deg", 16.0).unwrap().deg, -120.0);
    assert_eq!(parse_color("#f76").unwrap(), [1.0, 119.0 / 255.0, 102.0 / 255.0, 1.0]);
    assert_eq!(parse_color("rgba(0, 0, 0, 0.5)").unwrap()[3], 0.5);
    let hsl = parse_color("hsl(180deg 100% 80% / 0)").unwrap();
    assert!((hsl[0] - 0.6).abs() < 1e-9 && hsl[3] == 0.0);
}

#[test]
fn every_pattern_parses() {
    let patterns = parse_patterns(crate::cover::CSS);
    for id in crate::cover::PATTERN_IDS {
        let pattern = patterns.get(*id).unwrap_or_else(|| panic!("узор {id} не разобран"));
        if !matches!(*id, "city" | "dreams" | "tunnel") {
            assert!(!pattern.base.layers.is_empty(), "у узора {id} нет слоёв");
        }
    }
    let cubes = &patterns["cubes"].base.layers;
    assert_eq!(cubes.len(), 7);
    assert!(matches!(cubes[1].gradient.kind, Kind::Conic { from, .. } if from == -120.0));
    let zigzag = &patterns["zigzag"].base;
    assert_eq!(zigzag.layers[0].position.0.px, -40.0);
    assert_eq!(zigzag.layers[0].size.unwrap().0.px, 80.0);
    assert!(zigzag.color[3] == 1.0);
    assert_eq!(patterns["dreams"].overlay.layers.len(), 1);
}
