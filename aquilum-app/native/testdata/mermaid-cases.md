# Случаи для раскладки Mermaid

Каждый блок проверяет измерительная модель (`mermaid/check.rs`): узлы, рамки, заметки и подписи не накладываются, стрелки не проходят через чужие узлы и рамки.

```mermaid
flowchart TD
    A[Первый] --> B[Второй] --> C[Третий] --> D[Четвёртый] --> E[Пятый]
```

```mermaid
flowchart TD
    Root[Корень] --> A[Альфа]
    Root --> B[Бета]
    Root --> C[Гамма]
    Root --> D[Дельта]
    Root --> E[Эпсилон]
    Root --> F[Дзета]
```

```mermaid
flowchart TD
    A[Альфа] --> Z[Итог]
    B[Бета] --> Z
    C[Гамма] --> Z
    D[Дельта] --> Z
    E[Эпсилон] --> Z
```

```mermaid
flowchart TD
    Start([Начало]) --> Q{Выбор?}
    Q -- первый --> A[Путь А]
    Q -- второй --> B[Путь Б]
    Q -- третий --> C[Путь В]
    A --> End([Конец])
    B --> End
    C --> End
```

```mermaid
flowchart TD
    A --> B --> C --> A
    C --> D --> E --> C
    E --> F
```

```mermaid
flowchart TD
    A[Вход] --> B[Шаг 1] --> C[Шаг 2] --> D[Шаг 3] --> E[Шаг 4]
    E -- повторить --> A
    D -- назад --> B
```

```mermaid
flowchart TD
    A[Источник] -->|коротко| B[Приёмник]
    A -->|очень длинная подпись стрелки, которая точно не помещается в одну строку| C[Второй приёмник]
    B -->|первая строка<br/>вторая строка| D[Итог]
    C -- ещё одна подпись --> D
```

```mermaid
flowchart TD
    A[Клиент] -->|запрос| B[Сервер]
    A -->|повтор| B
    B <-->|сессия| C[База]
    C -.->|событие| A
```

```mermaid
flowchart LR
    A[Сбор] -->|сырые данные| B[Очистка] -->|таблица| C[Анализ] -->|выводы| D[Отчёт]
    B -->|ошибки| E[Журнал]
```

```mermaid
graph BT
    Root[Корень]
    A[Лист 1] --> P[Ветка 1]
    B[Лист 2] --> P
    C[Лист 3] --> Q[Ветка 2]
    D[Лист 4] --> Q
    P --> Root
    Q --> Root
```

```mermaid
flowchart RL
    A[Старт] -->|раз| B[Середина] -->|два| C[Финиш]
    A -->|напрямую| C
```

```mermaid
flowchart TD
    subgraph Front [Интерфейс]
        F1[Форма] --> F2[Проверка]
    end
    subgraph Back [Сервер]
        B1[Приём] --> B2[Запись]
    end
    F2 --> B1
    B2 -->|ответ| F1
```

```mermaid
flowchart TD
    subgraph Outer [Система]
        subgraph Middle [Модуль]
            subgraph Inner [Ядро]
                K1[Парсер] --> K2[Индекс]
            end
            M1[Кэш] --> K1
        end
        O1[Очередь] --> M1
    end
    X[Пользователь] --> O1
    K2 --> Y[Ответ]
    Y --> X
```

```mermaid
flowchart TD
    subgraph S1 [Хранилище]
        D1[Диск]
        D2[Облако]
    end
    A[Приложение] --> S1
    S1 --> B[Резервная копия]
```

```mermaid
flowchart TD
    subgraph Pipe [Конвейер]
        direction LR
        P1[Чтение] --> P2[Разбор] --> P3[Запись]
    end
    Start[Запуск] --> P1
    P3 --> Done[Готово]
```

```mermaid
flowchart TD
    A[Прямоугольник] --> B(Скруглённый)
    B --> C([Стадион])
    C --> D[[Подпрограмма]]
    D --> E[(База)]
    E --> F((Круг))
    F --> G{Ромб}
    G --> H{{Шестиугольник}}
    H --> I[/Параллелограмм/]
    I --> J[\Обратный\]
    J --> K[/Трапеция\]
    K --> L>Флажок]
```

```mermaid
flowchart TD
    A1[Один] --> B1[Альфа]
    A1 --> B2[Бета]
    A1 --> B3[Гамма]
    A2[Два] --> B1
    A2 --> B2
    A2 --> B3
    A3[Три] --> B1
    A3 --> B2
    A3 --> B3
```

```mermaid
flowchart TD
    R[Корень] --> A[Раздел А]
    R --> B[Раздел Б]
    R --> C[Раздел В]
    R --> D[Раздел Г]
    A --> A1[А1]
    A --> A2[А2]
    B --> B1[Б1]
    B --> B2[Б2]
    C --> C1[В1]
    C --> C2[В2]
    D --> D1[Г1]
    D --> D2[Г2]
```

```mermaid
flowchart TD
    A[Остров 1] --> B[Остров 1б]
    C[Остров 2] --> D[Остров 2б]
    E[Одиночка]
```

```mermaid
flowchart TD
    A[Опрос] -->|ждать| A
    A --> B[Обработка]
    B -->|снова| B
    B --> C[Итог]
```

```mermaid
flowchart TD
    A[Начало] --> B[Шаг] --> C[Шаг] --> D[Шаг] --> E[Конец]
    A -->|напрямую| E
    B -->|пропуск| D
```

```mermaid
flowchart TD
    Q{Тип файла?}
    Q -->|изображение в формате PNG или JPEG| I[Картинка]
    Q -->|документ PDF| P[Документ]
    Q -->|видео| V[Плеер]
    Q -->|архив zip или tar.gz| Z[Распаковка]
```

```mermaid
flowchart LR
    subgraph Left [Слева]
        L1[Л1] --> L2[Л2]
    end
    subgraph Right [Справа]
        R1[П1] --> R2[П2]
    end
    X[Посредник]
    L2 --> X --> R1
    R2 --> L1
```

```mermaid
flowchart TD
    subgraph A [Группа А]
        a1[а1] --> a2[а2]
    end
    subgraph B [Группа Б]
        b1[б1] --> b2[б2]
    end
    free1[Свободный] --> free2[Ещё свободный]
```

```mermaid
classDiagram
    class Animal {
        +String name
        +int age
        +eat() void
    }
    class Dog {
        +bark() void
    }
    class Cat {
        +meow() void
    }
    class Owner {
        +String name
    }
    Animal <|-- Dog
    Animal <|-- Cat
    Owner "1" --> "*" Animal : владеет
    Dog ..> Owner : слушается
```

```mermaid
classDiagram
    class Repository~T~ {
        <<interface>>
        +find(id) T
        +save(item T)
        +delete(id)
    }
    class NoteRepo
    class TagRepo
    class Service
    Repository <|.. NoteRepo
    Repository <|.. TagRepo
    Service o-- NoteRepo : хранит
    Service *-- TagRepo : содержит
    Service --> Logger
```

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Active: старт
    state Active {
        [*] --> Loading
        Loading --> Ready: загружено
        state Ready {
            [*] --> Viewing
            Viewing --> Editing: правка
            Editing --> Viewing: сохранить
        }
    }
    Active --> Idle: стоп
    Active --> Error: сбой
    Error --> [*]
```

```mermaid
stateDiagram-v2
    [*] --> Check
    state Check <<choice>>
    Check --> Fast: мало
    Check --> Slow: много
    state split <<fork>>
    Slow --> split
    split --> Left
    split --> Right
    state merge <<join>>
    Left --> merge
    Right --> merge
    merge --> Done
    Fast --> Done
    Done --> [*]
    note right of Fast: без очереди
    note left of Slow: в очереди
```

```mermaid
stateDiagram-v2
    [*] --> A
    A --> B: вперёд
    B --> C: вперёд
    C --> A: по кругу
    B --> A: назад
    C --> [*]
```

```mermaid
erDiagram
    USER ||--o{ NOTE : "пишет"
    NOTE ||--o{ TAG : "помечена"
    NOTE }o--|| VAULT : "лежит в"
    USER ||--|| PROFILE : "имеет"
    NOTE ||--o{ NOTE : "ссылается"
    VAULT {
        string id PK
        string name
    }
    USER {
        string id PK
        string email
    }
```
