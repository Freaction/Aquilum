# masonry (xilem 271a27a) для Aquilum

Копия `masonry` той же ревизии xilem, что и `masonry_core`, переведённая на parley 0.12. Основная ветка xilem пока на parley 0.8, а встроенным блокам с базовой линией (формулы в строке) нужен parley 0.12.

- Без `src/tests` и скриншотов; встроенные модули тестов виджетов оставлены как есть (собираются только для тестов самого masonry).
- `widgets/text_accessibility.rs` — интеграция с AccessKit, которую parley 0.12 убрал из библиотеки (PR linebender/parley#716) и перенёс в пример `examples/editor/src/accessibility.rs`. Взята оттуда (тег v0.12.0) и подогнана: `PlainEditorAccessibility::build_nodes` принимает `&PlainEditor` с уже готовой раскладкой (раскладка в Masonry всегда идёт до доступности), неиспользуемый `handle_action_request` убран. Работает с accesskit 0.24, как остальной Masonry.
- `Label`: `Layout::align` без ширины (с parley 0.9 выравнивание идёт по ширине разбивки).

Убрать копию, когда xilem перейдёт на parley 0.12.
