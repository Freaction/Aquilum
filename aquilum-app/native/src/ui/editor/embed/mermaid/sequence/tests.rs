use masonry::parley::Layout;
use mermaid_rs_renderer::{SequenceActivationKind, parse_mermaid};

use super::super::mermaid;
use super::plan::plan;

const BIG: &str = "sequenceDiagram
    actor U as Пользователь
    participant E as Редактор
    participant I as Индекс
    participant S as Синхронизация
    participant C as Облако
    U->>+E: Сохранить
    Note left of U: Ctrl+S
    E->>I: Обновить запись
    loop Пока есть файлы
        S->>C: Загрузить
        S->>S: Создать копию
    end
    Note over S,C: Повтор через 5 минут
    opt Есть сеть
        S->>C: Проверка
    end
    E-->>-U: Сохранено
    Note right of C: конец
";

#[test]
fn a_note_between_two_frames_stays_between_them() {
    let graph = parse_mermaid(BIG).unwrap().graph;
    let plan = plan(&graph);
    let note = plan.notes[1].0;
    let (looped, opt) = (plan.frames[0].1, plan.frames[1].1);
    assert!(looped.y1 < note.y0 && note.y1 < opt.y0, "{looped:?} {note:?} {opt:?}");
}

#[test]
fn a_minus_deactivates_the_sender() {
    let graph = parse_mermaid(BIG).unwrap().graph;
    let kinds: Vec<_> = graph.sequence_activations.iter().map(|a| (a.participant.as_str(), a.kind)).collect();
    assert_eq!(kinds, [("E", SequenceActivationKind::Activate), ("E", SequenceActivationKind::Deactivate)]);
}

#[test]
fn frames_wrap_their_messages() {
    let graph = parse_mermaid(BIG).unwrap().graph;
    let plan = plan(&graph);
    let x = |id: &String| plan.columns[graph.sequence_participants.iter().position(|p| p == id).unwrap()].x;
    for (k, r, _) in &plan.frames {
        let frame = &graph.sequence_frames[*k];
        for (e, (y, _)) in graph.edges.iter().zip(&plan.messages).take(frame.end_idx).skip(frame.start_idx) {
            assert!(r.y0 < *y && *y < r.y1, "{r:?} {y}");
            assert!(r.x0 < x(&e.from).min(x(&e.to)) && x(&e.from).max(x(&e.to)) < r.x1, "{r:?}");
        }
    }
}

#[test]
fn five_participants_with_side_notes_fit_a_desktop_column() {
    let drawing = mermaid(BIG, 620.0, &mut |_| Layout::default()).unwrap();
    assert_eq!(drawing.max_scroll(), 0.0);
}
