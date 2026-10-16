use super::{Column, Item, Laid, Line, Piece};

pub fn paginate(items: Vec<Item>, height: f64) -> Laid {
    let mut columns = vec![Column::default()];
    let mut y = 0.0;
    let mut total = 0.0;
    for (index, item) in items.iter().enumerate() {
        let (before, after) = item.margins();
        let start = |columns: &mut Vec<Column>, y: &mut f64| {
            columns.push(Column::default());
            *y = 0.0;
        };
        if y > 0.0 {
            y += before;
        }
        total += before;
        match item {
            Item::Text { lines, .. } => {
                for &Line { top, bottom, .. } in lines {
                    let tall = bottom - top;
                    if y + tall > height && y > 0.0 {
                        start(&mut columns, &mut y);
                    }
                    let column = columns.last_mut().expect("колонка");
                    match column.pieces.last_mut() {
                        Some(piece) if piece.item == index && (piece.to - top).abs() < 0.5 => piece.to = bottom,
                        _ => column.pieces.push(Piece { item: index, from: top, to: bottom, y }),
                    }
                    y += tall;
                    total += tall;
                }
            }
            Item::Image { size, .. } => {
                let tall = size.height.min(height);
                if y + tall > height && y > 0.0 {
                    start(&mut columns, &mut y);
                }
                columns.last_mut().expect("колонка").pieces.push(Piece { item: index, from: 0.0, to: tall, y });
                y += tall;
                total += tall;
            }
            Item::Rule => {
                if y + 1.0 > height && y > 0.0 {
                    start(&mut columns, &mut y);
                }
                columns.last_mut().expect("колонка").pieces.push(Piece { item: index, from: 0.0, to: 1.0, y });
                y += 1.0;
                total += 1.0;
            }
        }
        y += after;
        total += after;
    }
    if columns.len() > 1 && columns.last().is_some_and(|c| c.pieces.is_empty()) {
        columns.pop();
    }
    Laid { items, columns, height: total }
}
