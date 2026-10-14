const WIDEST: f64 = 480.0;
const NARROWEST: f64 = 96.0;

pub fn fit(natural: &[f64], room: f64) -> Vec<f64> {
    let capped: Vec<f64> = natural.iter().map(|w| w.min(WIDEST)).collect();
    let floor: Vec<f64> = capped.iter().map(|w| w.min(NARROWEST)).collect();
    if capped.iter().sum::<f64>() <= room { between(&capped, natural, room) } else { between(&floor, &capped, room) }
}

fn between(lo: &[f64], hi: &[f64], room: f64) -> Vec<f64> {
    let (low, high): (f64, f64) = (lo.iter().sum(), hi.iter().sum());
    let share = if high <= room { 1.0 } else { ((room - low) / (high - low)).max(0.0) };
    lo.iter().zip(hi).map(|(lo, hi)| lo + (hi - lo) * share).collect()
}

#[cfg(test)]
mod tests {
    use super::fit;

    fn rounded(widths: Vec<f64>) -> Vec<f64> {
        widths.into_iter().map(f64::round).collect()
    }

    #[test]
    fn a_table_is_as_wide_as_its_content_when_it_fits() {
        assert_eq!(fit(&[80.0, 120.0], 700.0), [80.0, 120.0]);
    }

    #[test]
    fn long_text_takes_the_spare_room_and_no_more() {
        assert_eq!(rounded(fit(&[80.0, 2000.0], 700.0)), [80.0, 620.0]);
        assert_eq!(fit(&[80.0, 600.0], 1200.0), [80.0, 600.0]);
    }

    #[test]
    fn wide_columns_give_way_before_narrow_ones() {
        assert_eq!(rounded(fit(&[80.0, 200.0, 3000.0], 700.0)), [80.0, 187.0, 433.0]);
    }

    #[test]
    fn columns_stop_at_their_narrowest_and_the_table_scrolls() {
        assert_eq!(fit(&[300.0; 10], 700.0), [96.0; 10]);
    }
}
