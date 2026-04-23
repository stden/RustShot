// RustShot — Product Guide & Technical Overview
// Canonical Typst source for the Russian documentation variant.

#set page(
  paper: "a4",
  margin: (x: 20mm, y: 22mm),
  header: context {
    if counter(page).get().first() > 1 [
      #set text(size: 9pt, fill: gray.darken(25%))
      🦀 RustShot v0.1.0
      #h(1fr)
      Руководство по продукту и технический обзор
      #h(1fr)
      #counter(page).display()
    ]
  },
  footer: [],
)

#set text(
  font: "New Computer Modern",
  size: 10.8pt,
  lang: "ru",
)

#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")

// Color palette
#let rust-orange = rgb("#F74C00")
#let rust-dark = rgb("#2B1400")
#let teal = rgb("#0D9488")
#let indigo = rgb("#4F46E5")
#let emerald = rgb("#059669")
#let rose = rgb("#E11D48")
#let amber = rgb("#D97706")
#let sky = rgb("#0284C7")
#let violet = rgb("#7C3AED")
#let slate = rgb("#475569")

#let app = "RustShot"

// Colored box helper
#let colorbox(body, color: teal, title: none) = {
  block(
    width: 100%,
    inset: 12pt,
    radius: 6pt,
    fill: color.lighten(92%),
    stroke: 1.5pt + color.lighten(50%),
  )[
    #if title != none [
      #text(weight: "bold", fill: color)[#title]
      #v(4pt)
    ]
    #body
  ]
}

// Diagram node helper
#let node(label, color: indigo, icon: "📦") = {
  box(
    inset: (x: 10pt, y: 6pt),
    radius: 8pt,
    fill: color.lighten(88%),
    stroke: 1.2pt + color.lighten(40%),
  )[#text(fill: color.darken(20%), weight: "bold")[#icon #label]]
}

// Arrow helper
#let arrow(color: slate) = {
  h(4pt)
  text(fill: color, size: 14pt)[→]
  h(4pt)
}

// ─────────────────────────────────────────────
// Title page
// ─────────────────────────────────────────────

#align(center + horizon)[
  #v(2em)
  #block(
    inset: 24pt,
    radius: 16pt,
    fill: rust-orange.lighten(94%),
    stroke: 2pt + rust-orange.lighten(60%),
  )[
    #text(size: 48pt)[🦀]
    #v(0.3em)
    #text(size: 36pt, weight: "bold", fill: rust-orange)[#app]
  ]
  #v(0.6em)
  #line(length: 58%, stroke: 2pt + rust-orange.lighten(60%))
  #v(0.7em)
  #text(size: 16pt, fill: slate)[📸 Утилита для скриншотов и 🎬 записи экрана]
  #v(0.8em)
  #text(size: 12pt, fill: indigo)[Руководство по продукту и технический обзор]
  #v(2em)
  #block(
    inset: 14pt,
    radius: 8pt,
    fill: indigo.lighten(95%),
    stroke: 1pt + indigo.lighten(70%),
  )[
    #text(size: 11pt, fill: indigo)[📋 Версия 0.1.0] #h(1.5em)
    #text(size: 11pt, fill: emerald)[🔓 MIT OR Apache-2.0] #h(1.5em)
    #text(size: 11pt, fill: teal)[🏗️ Собрано на Rust]
  ]
  #v(2em)
  #text(size: 10pt, fill: gray.darken(10%))[📂 Репозиторий: `stden/RustShot`]
  #v(3em)
  #text(size: 9.5pt, fill: gray)[⚙️ Сгенерировано из `RustShot.ru.typ`]
]

#pagebreak()

#outline(title: [📑 Содержание], depth: 3)

#pagebreak()

= 🎯 Обзор продукта

== 📌 Назначение

#colorbox(color: teal)[
  #app — это компактная настольная утилита для быстрых скриншотов и коротких записей экрана.
  Она построена вокруг прямого рабочего сценария: вызвать горячую клавишу или CLI-команду, захватить нужный фрагмент экрана и сразу вернуться к работе без тяжелого редактора или медиапакета.
]

Проект совмещает три практических роли в одном бинарнике:

#colorbox(color: indigo, title: "🔧 Три режима в одном бинарнике")[
  - 🖥️ *Интерфейс командной строки* — одноразовый захват и скрипты
  - 🔔 *Tray-приложение* — глобальные горячие клавиши и быстрый доступ с рабочего стола
  - 🎛️ *Внутренние вспомогательные режимы* — оверлеи выбора области и подготовка записи
]

Реализация сознательно предпочитает узкий, надежный набор возможностей вместо большой UI-поверхности.
Это видно по всей кодовой базе: небольшие helper-функции, явная работа с путями, простое JSON-хранилище настроек и нативные API там, где они делают инструмент предсказуемее.

== ✨ Основные возможности

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: emerald.lighten(94%),
    stroke: 1.2pt + emerald.lighten(50%),
  )[
    #grid(
      columns: (1fr, 1fr),
      gutter: 8pt,
      [📸 Захват скриншотов в `png`, `jpg`, `bmp`],
      [🎯 Выбор области через overlay-сценарий],
      [🪟 Захват активного окна в Windows],
      [🖥️ Поддержка нескольких мониторов],
      [🎬 Запись экрана в H.264 MP4 (Windows)],
      [📋 Копирование в буфер обмена для tray-снимков области],
      [🔔 Системный трей с глобальными горячими клавишами],
      [⚙️ Постоянные настройки каталогов, формата и FPS],
    )
  ],
  caption: [Обзор основных возможностей],
)

= 🔄 Пользовательские сценарии

== 🔔 Tray-режим

Запуск `rustshot` без аргументов включает tray-режим.
Это основной интерактивный сценарий приложения, рассчитанный на повседневную работу на рабочем столе.

// Tray workflow diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: sky.lighten(95%),
    stroke: 1.5pt + sky.lighten(55%),
  )[
    #align(center)[
      #text(fill: sky, weight: "bold", size: 11pt)[🔄 Диаграмма tray-сценария]
      #v(8pt)
      #node("Запуск", color: emerald, icon: "🚀")
      #arrow(color: emerald)
      #node("Значок в трее", color: indigo, icon: "🔔")
      #arrow(color: indigo)
      #node("Горячая клавиша / меню", color: amber, icon: "⌨️")
      #v(10pt)
      #h(4em)
      #node("Захват", color: teal, icon: "📸")
      #arrow(color: teal)
      #node("Сохранение / буфер", color: emerald, icon: "💾")
    ]
  ],
  caption: [Жизненный цикл tray-режима],
)

#v(8pt)

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt,
    align: (left, left),
    fill: (col, row) => if row == 0 { indigo.lighten(88%) } else if calc.odd(row) { indigo.lighten(96%) } else { white },
    [*⌨️ Действие*], [*📋 Результат*],
    [🖨️ PrintScreen], [Открыть оверлей выделения и скопировать область в буфер обмена],
    [🪟 Alt+PrintScreen], [Захватить активное окно и сохранить в папку скриншотов (Windows)],
    [🎬 Ctrl+Shift+R], [Начать или остановить запись области (Windows)],
    [📸 Трей → Снимок экрана], [Аналогично горячей клавише PrintScreen],
    [🪟 Трей → Захват окна], [Аналогично горячей клавише Alt+PrintScreen (Windows)],
    [📂 Трей → Открыть папку], [Открыть настроенную папку скриншотов],
    [🎬 Трей → Старт/Стоп записи], [Аналогично горячей клавише Ctrl+Shift+R (Windows)],
    [⚙️ Трей → Настройки], [Открыть диалог настроек (нативный на Windows, редактор на других платформах)],
    [🚪 Трей → Выход], [Остановить приложение],
  ),
  caption: [Стандартный tray-сценарий],
)

Tray-режим реализован как долгоживущий цикл обработки событий.
Он владеет значком в трее, регистрацией глобальных hotkey и управлением вспомогательными дочерними процессами.

== 🖥️ CLI-режим

CLI-режим нужен для прямых команд захвата и сценариев автоматизации.
Он включается, когда бинарник запускают с `--cli` или с другими флагами захвата, например `--list`, `--screen`, `--output`, `--format` или `--window`.

#colorbox(color: violet, title: "💻 Типичные примеры")[
  ```bash
  rustshot --cli
  rustshot --list
  rustshot --screen 1 --output monitor2.png
  rustshot --window
  rustshot --format jpg --output shot.jpg
  ```
]

CLI-захват пишет выходной файл либо:

- по пути, указанному пользователем через `--output`, либо
- в настроенный каталог скриншотов с автоматически сгенерированным именем с отметкой времени

== 🎛️ Вспомогательные режимы выбора и записи

Бинарник также содержит скрытые режимы, которые tray-режим использует изнутри:

- `--select` для выбора области скриншота
- `--select-record` для выбора области записи

#colorbox(color: amber, title: "💡 Почему так устроено")[
  Эти режимы отделяют короткоживущий overlay-сценарий от долгоживущего tray-процесса.
  Такое разделение уменьшает объем общего состояния и упрощает рассуждение об ошибках.
]

= 📖 Справка по командной строке

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: slate.lighten(94%),
    stroke: 1.2pt + slate.lighten(50%),
  )[
    ```text
    rustshot [OPTIONS]

    Options:
      --cli               Run in CLI mode (single capture, then exit)
      -w, --window        Capture the active window
      -o, --output <PATH> Output file path
      -f, --format <FMT>  Image format: png, jpg, bmp
      -s, --screen <IDX>  Screen index (0-based)
      -l, --list          List available screens
      -h, --help          Print help
      -V, --version       Print version
    ```
  ],
  caption: [Справка по CLI-опциям],
)

== 📝 Частые примеры

```bash
# 📸 Снять экран по умолчанию в PNG
rustshot --cli

# 💾 Сохранить в явный файл
rustshot --output my_screenshot.png

# 🖥️ Снять экран с индексом 1 как JPEG
rustshot --screen 1 --format jpg --output monitor2.jpg

# 🪟 Снять активное окно (только Windows)
rustshot --window

# 📋 Показать список доступных экранов
rustshot --list
```

= 📷 Поведение захвата и записи

== 📸 Скриншоты

RustShot поддерживает несколько сценариев скриншотов:

#colorbox(color: teal, title: "📸 Сценарии скриншотов")[
  - 🔔 Захват выбранной области через `PrintScreen` в tray-режиме
  - 🪟 Захват активного окна через `Alt+PrintScreen` на Windows
  - 🖥️ CLI-захват конкретного экрана или экрана по умолчанию
]

Сценарий выбора области в tray-режиме копирует обрезанное изображение в буфер обмена.
CLI-захваты и снимки активного окна сохраняют файлы на диск.

Если выходной путь явно не задан, RustShot генерирует имена в формате:

```text
screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>
```

Дополнительный суффикс в наносекундах уменьшает риск коллизий при серии быстрых снимков.

== 🎬 Запись экрана

Запись экрана сейчас доступна только на Windows.

// Recording pipeline diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: rose.lighten(95%),
    stroke: 1.5pt + rose.lighten(55%),
  )[
    #align(center)[
      #text(fill: rose, weight: "bold", size: 11pt)[🎬 Конвейер записи]
      #v(8pt)
      #node("Hotkey", color: amber, icon: "⌨️")
      #arrow(color: amber)
      #node("Выбор области", color: indigo, icon: "🎯")
      #arrow(color: indigo)
      #node("Захват кадров", color: teal, icon: "🖼️")
      #v(10pt)
      #h(4em)
      #node("Кодирование H.264", color: rose, icon: "⚡")
      #arrow(color: rose)
      #node("MP4-файл", color: emerald, icon: "💾")
    ]
  ],
  caption: [Поток обработки записи],
)

#v(6pt)

Этапы сценария:

+ Запустить запись через `Ctrl+Shift+R` или меню в трее
+ Протянуть область в оверлее выбора
+ RustShot захватывает кадры этой области в фоновом потоке
+ Остановить запись той же горячей клавишей
+ MP4-файл финализируется в каталоге recordings

Записи кодируются как H.264 в контейнере MP4 через Windows Media Foundation.
Это делает результат совместимым с обычными проигрывателями и привычными сценариями обмена файлами без подключения тяжелого внешнего медиастека.

== 📂 Каталоги вывода

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt,
    align: (left, left),
    fill: (col, row) => if row == 0 { teal.lighten(88%) } else if calc.odd(row) { teal.lighten(96%) } else { white },
    [*📁 Тип*], [*📍 Расположение*],
    [📸 Скриншоты], [`Pictures/RustShot`],
    [🎬 Записи], [`Pictures/RustShot/recordings`],
    [⚙️ Настройки], [config dir / `RustShot/settings.json`],
    [📜 Логи], [config dir / `RustShot/rustshot.log`],
  ),
  caption: [Стандартные каталоги вывода],
)

= ⚙️ Настройки

== 📋 Сохраняемые поля

#figure(
  table(
    columns: (auto, auto, 1fr),
    inset: 8pt,
    align: (left, left, left),
    fill: (col, row) => if row == 0 { violet.lighten(88%) } else if calc.odd(row) { violet.lighten(96%) } else { white },
    [*🔑 Поле*], [*📌 По умолчанию*], [*📝 Описание*],
    [`screenshots_dir`], [`<Pictures>/RustShot`], [Каталог для сохраняемых скриншотов],
    [`recordings_dir`], [`<Pictures>/RustShot/recordings`], [Каталог для MP4-записей],
    [`default_format`], [`"png"`], [Формат по умолчанию для CLI-скриншотов и снимков активного окна],
    [`recording_fps`], [`30`], [Частота кадров записи],
    [`auto_start`], [`true`], [Нужно ли регистрировать RustShot для автозапуска при входе в Windows],
  ),
  caption: [⚙️ Сохраняемые настройки],
)

== 💾 Формат хранения

Настройки хранятся как JSON-текст, но проект сознательно использует небольшой ручной парсер вместо `serde`.
Это уменьшает поверхность зависимостей и позволяет явно обрабатывать частично поврежденные или неполные входные данные.

#colorbox(color: amber, title: "🛡️ Обработка неверных значений")[
  - ❌ Неизвестный `default_format` → откат к `"png"`
  - ❌ `recording_fps`, равный `0` → откат к `30`
  - ❌ Отсутствующие ключи → сохраняются значения по умолчанию
  - ❌ Нечитаемый или отсутствующий файл → полная конфигурация по умолчанию
]

== 🖥️ Интерфейс настроек

- 🪟 На Windows меню трея открывает нативный Win32-диалог настроек
- 🐧 На Linux и 🍎 macOS файл настроек открывается в редакторе по умолчанию

Диалог Windows позволяет выбирать каталоги, менять формат по умолчанию, редактировать FPS и переключать автозапуск.

= 🌍 Поддержка платформ

#figure(
  table(
    columns: (auto, auto, auto, auto),
    inset: 8pt,
    align: (left, center, center, center),
    fill: (col, row) => if row == 0 { indigo.lighten(88%) } else if calc.odd(row) { indigo.lighten(96%) } else { white },
    [*🔧 Возможность*], [*🪟 Windows*], [*🐧 Linux*], [*🍎 macOS*],
    [CLI-скриншоты], [✅], [✅], [✅],
    [Tray-режим], [✅], [✅], [✅],
    [Захват области в буфер обмена], [✅], [✅], [✅],
    [Захват активного окна], [✅], [❌], [❌],
    [Запись экрана], [✅], [❌], [❌],
    [Интеграция с автозапуском], [✅], [❌], [❌],
  ),
  caption: [🌍 Поддержка возможностей по платформам],
)

Windows имеет самый широкий набор возможностей, потому что запись, захват активного окна и автозапуск опираются на нативные Windows API.
Linux сейчас зависит от библиотек X11, которые использует стек скриншотов в проекте.

= 🏗️ Обзор архитектуры

== 📐 Общая схема

// Architecture diagram
#figure(
  block(
    width: 100%,
    inset: 16pt,
    radius: 12pt,
    fill: indigo.lighten(96%),
    stroke: 1.5pt + indigo.lighten(55%),
  )[
    #align(center)[
      #text(fill: indigo, weight: "bold", size: 12pt)[🏗️ Архитектура модулей]
      #v(12pt)

      // Top layer
      #node("main.rs", color: rust-orange, icon: "🚀")
      #v(8pt)

      // Middle layer
      #grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 8pt,
        align(center)[#node("capture.rs", color: teal, icon: "📸")],
        align(center)[#node("selection.rs", color: indigo, icon: "🎯")],
        align(center)[#node("record.rs", color: rose, icon: "🎬")],
      )
      #v(8pt)

      // Bottom layer
      #grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 8pt,
        align(center)[#node("settings.rs", color: violet, icon: "⚙️")],
        align(center)[#node("tray.rs", color: emerald, icon: "🔔")],
        align(center)[#node("settings_dialog.rs", color: amber, icon: "🖥️")],
      )
      #v(8pt)

      #node("lib.rs", color: slate, icon: "📦")
    ]
  ],
  caption: [Диаграмма зависимостей модулей],
)

== 🚀 `src/main.rs`

`main.rs` — это слой оркестрации.
Он разбирает аргументы, выбирает режим приложения, инициализирует логирование, настраивает Windows DPI awareness, запускает tray-режим и порождает вспомогательные подпроцессы для выбора области и записи.

В этом файле также живут CLI-сценарий, перенос данных в буфер обмена для tray-скриншотов области и синхронизация автозапуска на Windows.

== 📸 `src/capture.rs`

Этот модуль отвечает за захват скриншотов и сохранение изображений.
В его обязанности входят:

- 🖥️ Определение индекса экрана
- 📂 Генерация выходного пути по умолчанию
- 📁 Создание родительских каталогов
- 🖼️ Кодирование изображений в PNG, JPEG или BMP
- 🪟 Захват активного окна на Windows
- 🧪 Небольшие тестируемые helper-функции для преобразования пикселей и поддержки бенчмарков

Для тестирования он использует внутренний инжектируемый pipeline, поэтому логику захвата можно проверять без реального железа в каждом unit-тесте.

== 🎯 `src/selection.rs`

Это самый чувствительный к раскладке модуль проекта.
Он отвечает за:

- 🖼️ Поведение overlay-окна выбора
- 🖱️ Преобразование жестов перетаскивания в `Region`
- 🖥️ Определение предпочтительного монитора
- 📐 DPI-aware рассуждение о переходе от логических координат к физическим
- 📸 Вспомогательные сценарии захвата для скриншотов и выбора области записи

Сложность здесь возникает в первую очередь из-за реальных многомониторных систем координат, а не только из-за визуальной части UI.

== 🎬 `src/record.rs`

Этот модуль содержит реализацию записи для Windows.
Он проверяет запрошенный FPS, запускает фоновый поток записи, настраивает Media Foundation, захватывает кадры, переводит данные RGBA в формат BGRA bottom-up, записывает sample-объекты и финализирует MP4-файл.

#colorbox(color: rose, title: "🔧 Ключевые helper-функции")[
  - `frame_duration_hns` — длительность кадра в единицах по 100 нс
  - `round_to_even` — выравнивание размеров для H.264
  - `pack_u32_pair` — упаковка пар атрибутов
  - `rgba_to_bgra_bottom_up` — преобразование формата пикселей
  - `validate_capture_region` — проверка корректности области захвата
]

== ⚙️ `src/settings.rs` and `src/settings_dialog.rs`

`settings.rs` отвечает за сохранение, нормализацию и OS-specific helper-функции для путей.
`settings_dialog.rs` предоставляет нативный Windows-диалог, позволяющий менять настройки без ручного редактирования JSON.

Вместе эти модули держат конфигурацию простой, но достаточно практичной для реальных пользователей.

== 🔔 `src/tray.rs`

Модуль трея строит значок приложения и структуру меню.
Он экспортирует стабильные строковые идентификаторы действий меню и программно собирает RGBA-иконку 32×32, которую использует запущенное приложение.

= 🛡️ Обработка ошибок и диагностика

#colorbox(color: rose, title: "⚠️ Стратегия обработки ошибок")[
  RustShot последовательно использует `Result<T, String>` по всему проекту.
  Это делает распространение ошибок прямым, а пользовательские сообщения читаемыми.
]

Примеры граничных ошибок, которые обрабатываются явно:

- 🖥️ Сбои перечисления экранов
- 📸 Ошибки захвата
- 📁 Ошибки создания каталогов
- 💾 Ошибки записи файлов
- 📋 Проблемы доступа к буферу обмена
- 🪟 Сбои COM и Media Foundation на Windows

Логирование инициализируется рано и пишет данные в пользовательскую область config.
Это особенно полезно для tray-режима и падений дочерних процессов, когда постоянной терминальной сессии нет.

= 🧪 Сборка, тестирование и бенчмарки

== ⚡ Частые команды

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: emerald.lighten(94%),
    stroke: 1.2pt + emerald.lighten(50%),
  )[
    ```bash
    cargo check           # ✅ Проверить компиляцию
    cargo test            # 🧪 Запустить все тесты
    cargo fmt             # 🎨 Отформатировать код
    cargo clippy          # 🔍 Прогнать линтер
    cargo build --release # 📦 Собрать релиз
    cargo bench           # ⚡ Запустить бенчмарки
    ```
  ],
  caption: [Команды для разработки],
)

== 📊 Ожидания по проверке

Репозиторий придерживается test-first подхода и делает упор на небольшие, сфокусированные тесты.

// Test coverage diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: emerald.lighten(95%),
    stroke: 1.5pt + emerald.lighten(55%),
  )[
    #align(center)[
      #text(fill: emerald, weight: "bold", size: 11pt)[🧪 Зоны покрытия тестами]
      #v(10pt)
      #grid(
        columns: (1fr, 1fr),
        gutter: 10pt,
        colorbox(color: teal)[📂 Генерация и нормализация путей],
        colorbox(color: indigo)[⚙️ Разбор настроек и поврежденные входные данные],
        colorbox(color: violet)[🖥️ Определение экранов и индексов мониторов],
        colorbox(color: amber)[📐 DPI и helper-геометрия областей],
        colorbox(color: rose)[🎬 Тайминги записи и преобразование пикселей],
        colorbox(color: emerald)[🔔 Пиксели tray-иконки и константы меню],
      )
    ]
  ],
  caption: [Карта покрытия тестов — 2 300+ автоматических тестов],
)

Бенчмарки покрывают те helper-функции, которые важны в повторяющихся desktop-сценариях: преобразование пикселей, подготовку изображений, JSON-парсинг и координатную логику.

= 📦 Релиз и упаковка

Репозиторий содержит документацию по установщику и release pipeline рядом с кодом приложения.
Это соответствует реальному объему проекта: настольная утилита полезна только тогда, когда ее можно собрать, установить и сопровождать.

#colorbox(color: sky, title: "📚 Сопутствующие материалы")[
  - 📦 Руководство по установщику
  - 🔧 Документация по Inno Setup
  - 🚀 Заметки по release pipeline
  - ⚡ Артефакты бенчмарков и тестирования
  - 📋 Планы проекта и документы review
]

= ⚠️ Практические ограничения

Самые чувствительные инженерные зоны:

#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: amber.lighten(94%),
    stroke: 1.5pt + amber.lighten(50%),
  )[
    #grid(
      columns: (1fr, 1fr),
      gutter: 10pt,
      colorbox(color: rose)[🖥️ Многомониторная раскладка и преобразование DPI],
      colorbox(color: amber)[🎬 Жизненный цикл объектов записи и финализация на Windows],
      colorbox(color: violet)[🪟 Platform-specific интеграция с рабочим столом],
      colorbox(color: indigo)[📋 Поведение буфера обмена во вспомогательных подпроцессах],
    )
    #v(6pt)
    #align(center)[
      #colorbox(color: teal)[🔔 Необходимо сохранить отзывчивость tray-сценария, пока запускаются короткоживущие helper-процессы]
    ]
  ],
  caption: [⚠️ Инженерные области повышенного риска],
)

Именно здесь тонкие регрессии наиболее вероятны, даже если видимая возможность кажется простой.

= 🏁 Заключение

#colorbox(color: rust-orange, title: "🦀 Что такое RustShot")[
  RustShot намеренно ограничен по объему.
  Это не полноценный редактор изображений и не видеостудия.
  Его ценность в том, что он быстро и предсказуемо выполняет небольшой набор задач по захвату экрана и при этом достаточно глубоко интегрируется с нативной средой рабочего стола.
]

Это сочетание делает проект полезным сразу в двух направлениях:

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  colorbox(color: emerald, title: "👤 Для пользователей")[
    Быстрая и легкая утилита для скриншотов и записей, которая не мешает основной работе.
  ],
  colorbox(color: indigo, title: "🦀 Для разработчиков")[
    Компактный пример реального desktop-системного программирования на Rust с крупным набором тестов.
  ],
)

// ═══════════════════════════════════════════════════════════════════
// Helper boxes for the tutorial
// ═══════════════════════════════════════════════════════════════════

#let funfact(body) = block(width: 100%, inset: 12pt, radius: 8pt, fill: rgb("#FFF7ED"), stroke: 1.5pt + rgb("#FB923C"))[#text(weight: "bold", fill: rgb("#C2410C"))[🤓 Занятный факт:] #body]

#let joke(body) = block(width: 100%, inset: 12pt, radius: 8pt, fill: rgb("#FEF2F2"), stroke: 1.5pt + rgb("#F87171"))[#text(weight: "bold", fill: rgb("#B91C1C"))[😄 Программистский юмор:] #body]

#let protip(body) = block(width: 100%, inset: 12pt, radius: 8pt, fill: rgb("#ECFDF5"), stroke: 1.5pt + rgb("#34D399"))[#text(weight: "bold", fill: rgb("#065F46"))[💡 Pro tip:] #body]

#let pixel(r, g, b) = box(width: 14pt, height: 14pt, fill: rgb(r, g, b), stroke: 0.5pt + gray)

#let memcell(label, color: slate) = box(inset: (x: 4pt, y: 3pt), fill: color.lighten(85%), stroke: 0.8pt + color.lighten(40%))[#text(size: 8pt, fill: color.darken(20%), weight: "bold")[#label]]

#pagebreak()

// ═══════════════════════════════════════════════════════════════════
// ЧАСТЬ II — Учебник по Rust
// ═══════════════════════════════════════════════════════════════════

= 🦀 Учебник по Rust: Best Practices

#colorbox(color: rust-orange, title: "📖 О чём этот раздел")[
  Практический учебник по идиоматическому Rust на реальных примерах из RustShot.
  С юмором, картинками и конкретным кодом — чтобы было не только полезно, но и весело 🎉
]

#joke[
  — Почему Rust-программисты не болеют? \
  — Потому что у них нет *null pointer* 🦀💪
]

== 🏗️ Владение и заимствование

#colorbox(color: teal, title: "✅ Передача по ссылке — экономим память")[
  ```rust
  pub fn normalize_default_format(value: &str) -> &'static str {
      match value {
          "png" | "jpg" | "bmp" => value,
          _ => "png",
      }
  }
  ```
]

#colorbox(color: rose, title: "❌ Ненужное клонирование — тратим память")[
  ```rust
  fn normalize(value: String) -> String {
      if value == "png" { value } else { "png".to_string() }
  }
  ```
]

#funfact[
  В Rust невозможно использовать значение после `move`. Компилятор ловит это *до* запуска программы. В C++ аналогичный баг мог бы жить в проде месяцами 👻
]

== 🛡️ Обработка ошибок

#colorbox(color: teal, title: "✅ Оператор ? — элегантный проброс ошибок")[
  ```rust
  pub fn ensure_parent_directory(path: &Path) -> Result<(), String> {
      if let Some(parent) = path.parent() {
          fs::create_dir_all(parent)
              .map_err(|e| format!("Failed to create directory: {e}"))?;
      }
      Ok(())
  }
  ```
]

#colorbox(color: rose, title: "❌ unwrap() в продакшн-коде")[
  ```rust
  let file = File::open(path).unwrap(); // 💀 паника при ошибке
  ```
]

#joke[
  `unwrap()` — это как прыжок с парашютом без запасного. \
  В тестах — адреналин 🪂. В продакшне — некролог 💀
]

== 📐 Enum вместо строк

#colorbox(color: teal, title: "✅ Компилятор проверяет полноту match")[
  ```rust
  #[derive(Debug, Clone, Copy, Default)]
  pub enum Format {
      #[default] Png,
      Jpg,
      Bmp,
  }
  ```
]

#funfact[
  Компилятор Rust — ваш лучший друг 🤝. Он ругается не потому что злой, а потому что заботится. Как мама, только быстрее и без эмоций.
]

== 🧪 Тестирование

RustShot содержит *2 450+* тестов 🎉 — больше, чем строк продакшн-кода!

#joke[
  «Код без тестов — как машина без тормозов. \
  Быстро? Да. Далеко? Нет.» 🏎️💥
]

#colorbox(color: indigo, title: "🔄 Рабочий цикл TDD")[
  + ✏️ Написать тест
  + ❌ Убедиться, что он _падает_
  + ✅ Написать минимальный код
  + 🎉 Убедиться, что тест _проходит_
  + 🧪 Запустить *все* тесты
]

#colorbox(color: teal, title: "✅ DI через замыкания — тестируем без Windows")[
  ```rust
  fn sync_autorun_with_handlers<Enable, Disable>(
      enabled: bool,
      enable_autorun: Enable,
      disable_autorun: Disable,
  ) { if enabled { enable_autorun(); } else { disable_autorun(); } }
  ```
]

#protip[
  Если тест можно написать за 30 секунд — напишите его. Баг, который он поймает через полгода, мог бы стоить вам 3 часа дебага 🕵️
]

== ⚡ Производительность

#colorbox(color: teal, title: "✅ BufWriter — меньше системных вызовов")[
  ```rust
  let file = File::create(path)?;
  let mut writer = BufWriter::new(file); // 125 вызовов вместо 1000!
  ```
]

#colorbox(color: teal, title: "✅ Итераторы — zero-cost abstractions")[
  ```rust
  let raw: Vec<u8> = image.pixels()
      .flat_map(|p| [p[0], p[1], p[2]])
      .collect();
  ```
]

#protip[
  Если вы пишете `for i in 0..vec.len() { vec[i] }` — компилятор грустит 😢. Используйте `for item in &vec` — и он улыбнётся 😊, и bounds-check уберёт!
]

#pagebreak()

= 🖼️ Работа с графикой

== 📸 Как устроен скриншот

Скриншот — двумерный массив пикселей из видеопамяти GPU 🖥️→🧠

#colorbox(color: indigo, title: "🔢 Структура пикселя RGBA")[
  Каждый пиксель = 4 байта: *R* 🔴 *G* 🟢 *B* 🔵 *A* ⬜ (прозрачность)
]

// Pixel grid visual
#figure(
  block(width: 100%, inset: 14pt, radius: 10pt, fill: white, stroke: 1pt + gray.lighten(50%))[
    #align(center)[
      #text(fill: slate, weight: "bold", size: 10pt)[🎨 Сетка пикселей 4×4]
      #v(8pt)
      #grid(
        columns: 4, gutter: 2pt,
        pixel(255, 0, 0), pixel(255, 128, 0), pixel(255, 255, 0), pixel(0, 255, 0),
        pixel(0, 255, 255), pixel(0, 128, 255), pixel(0, 0, 255), pixel(128, 0, 255),
        pixel(255, 0, 255), pixel(255, 128, 128), pixel(128, 255, 128), pixel(128, 128, 255),
        pixel(64, 64, 64), pixel(128, 128, 128), pixel(192, 192, 192), pixel(255, 255, 255),
      )
      #v(4pt)
      #text(size: 8pt, fill: gray)[Каждый квадрат = 1 пиксель (4 байта)]
    ]
  ],
  caption: [16 пикселей = 64 байта в памяти],
)

// RGBA memory layout
#figure(
  block(width: 100%, inset: 14pt, radius: 10pt, fill: indigo.lighten(97%), stroke: 1pt + indigo.lighten(60%))[
    #align(center)[
      #text(fill: indigo, weight: "bold", size: 10pt)[🧠 Три пикселя в памяти (🔴🟢🔵)]
      #v(8pt)
      #memcell("R=255", color: rose) #memcell("G=0", color: emerald) #memcell("B=0", color: sky) #memcell("A=255", color: slate)
      #h(4pt)
      #memcell("R=0", color: rose) #memcell("G=255", color: emerald) #memcell("B=0", color: sky) #memcell("A=255", color: slate)
      #h(4pt)
      #memcell("R=0", color: rose) #memcell("G=0", color: emerald) #memcell("B=255", color: sky) #memcell("A=255", color: slate)
    ]
  ],
  caption: [Непрерывный массив: каждые 4 байта = 1 пиксель],
)

#funfact[
  Full HD скриншот (1920×1080) занимает ~8 МБ в памяти без сжатия. Это ~2 миллиона пикселей × 4 байта. 4K — уже 32 МБ! 🤯
]

== 📐 DPI: логические vs физические пиксели

#colorbox(color: amber, title: "⚠️ Самая частая ловушка при работе с экранами")[
  При масштабе 150% монитор 1920×1080: \
  *Логическое* = 1280×720 (ОС) | *Физическое* = 1920×1080 (реальные пиксели) \
  Захватывайте *физические*, иначе скриншот будет размытым! 🔍
]

// DPI visual
#figure(
  block(width: 100%, inset: 14pt, radius: 10pt, fill: amber.lighten(96%), stroke: 1pt + amber.lighten(50%))[
    #align(center)[
      #text(fill: amber, weight: "bold", size: 10pt)[📐 Масштаб ×1.5]
      #v(8pt)
      #grid(
        columns: (1fr, auto, 1fr), gutter: 12pt,
        align(center)[
          #text(size: 9pt, weight: "bold", fill: rose)[Логические]
          #v(4pt)
          #grid(columns: 3, gutter: 1pt,
            ..range(9).map(_ => box(width: 18pt, height: 18pt, fill: rose.lighten(70%), stroke: 0.5pt + rose)))
          #v(4pt)
          #text(size: 8pt, fill: slate)[3×3 крупных]
        ],
        align(center + horizon)[#text(size: 14pt, fill: amber)[×1.5]],
        align(center)[
          #text(size: 9pt, weight: "bold", fill: teal)[Физические]
          #v(4pt)
          #grid(columns: 5, gutter: 0.5pt,
            ..range(20).map(_ => box(width: 11pt, height: 11pt, fill: teal.lighten(70%), stroke: 0.3pt + teal)))
          #v(4pt)
          #text(size: 8pt, fill: slate)[5×4 мелких]
        ],
      )
    ]
  ],
  caption: [Больше физических пикселей = чётче изображение],
)

== 🎬 MP4 и H.264

#colorbox(color: rose, title: "📦 MP4 — контейнер, не кодек!")[
  MP4 — это «коробка» 📦, внутри которой лежат: \
  🎬 Видео (H.264) + 🔊 Аудио (AAC) + 📋 Метаданные
]

// MP4 structure visual
#figure(
  block(width: 100%, inset: 14pt, radius: 10pt, fill: rose.lighten(97%), stroke: 1pt + rose.lighten(60%))[
    #align(center)[
      #text(fill: rose, weight: "bold", size: 10pt)[📦 Структура MP4 файла]
      #v(8pt)
      #box(width: 90%, inset: 6pt, fill: amber.lighten(85%), stroke: 1pt + amber, radius: 4pt)[
        #text(size: 9pt, weight: "bold", fill: amber.darken(20%))[ftyp] #h(1fr) #text(size: 8pt, fill: slate)[~20 байт — «я MP4!»]]
      #v(3pt)
      #box(width: 90%, inset: 8pt, fill: indigo.lighten(92%), stroke: 1pt + indigo.lighten(50%), radius: 4pt)[
        #text(size: 9pt, weight: "bold", fill: indigo.darken(20%))[moov 🗂️] #h(1fr) #text(size: 8pt, fill: slate)[метаданные]
        #v(4pt)
        #box(width: 100%, inset: 4pt, fill: violet.lighten(90%), stroke: 0.5pt + violet.lighten(50%), radius: 3pt)[
          #text(size: 8pt, fill: violet)[mvhd — длительность, FPS | trak → stbl — таблица кадров]]
      ]
      #v(3pt)
      #box(width: 90%, inset: 6pt, fill: emerald.lighten(85%), stroke: 1pt + emerald, radius: 4pt)[
        #text(size: 9pt, weight: "bold", fill: emerald.darken(20%))[mdat 🎬] #h(1fr) #text(size: 8pt, fill: slate)[95%+ файла — сжатые кадры]]
    ]
  ],
  caption: [MP4 = вложенные «коробки» (atoms)],
)

#funfact[
  H.264 сжимает видео в ~100 раз! 1 минута Full HD 30fps без сжатия = 14 ГБ. С H.264 = ~150 МБ. Магия математики 🧙‍♂️
]

=== 🔄 RGBA → BGRA Bottom-Up

#colorbox(color: amber, title: "⚠️ Наследие 1985 года")[
  Windows ожидает *BGRA* (каналы местами!) и *bottom-up* (строки наоборот!).
]

// RGBA vs BGRA visual
#figure(
  block(width: 100%, inset: 14pt, radius: 10pt, fill: amber.lighten(96%), stroke: 1pt + amber.lighten(50%))[
    #align(center)[
      #text(fill: amber, weight: "bold", size: 10pt)[🔄 Двойная трансформация]
      #v(8pt)
      #grid(columns: (1fr, auto, 1fr), gutter: 8pt,
        align(center)[
          #text(size: 9pt, weight: "bold", fill: teal)[RGBA]
          #v(4pt)
          #memcell("R", color: rose) #memcell("G", color: emerald) #memcell("B", color: sky) #memcell("A", color: slate)
        ],
        align(center + horizon)[#text(size: 20pt, fill: amber)[→]],
        align(center)[
          #text(size: 9pt, weight: "bold", fill: indigo)[BGRA]
          #v(4pt)
          #memcell("B", color: sky) #memcell("G", color: emerald) #memcell("R", color: rose) #memcell("A", color: slate)
        ],
      )
      #v(8pt)
      #grid(columns: (1fr, auto, 1fr), gutter: 8pt,
        align(center)[
          #text(size: 9pt, weight: "bold", fill: teal)[Top-down ↓]
          #v(4pt)
          #box(inset: 3pt, fill: rose.lighten(80%), stroke: 0.5pt + rose, radius: 2pt)[#text(size: 8pt)[Строка 0]]
          #v(1pt)
          #box(inset: 3pt, fill: emerald.lighten(80%), stroke: 0.5pt + emerald, radius: 2pt)[#text(size: 8pt)[Строка 1]]
          #v(1pt)
          #box(inset: 3pt, fill: sky.lighten(80%), stroke: 0.5pt + sky, radius: 2pt)[#text(size: 8pt)[Строка 2]]
        ],
        align(center + horizon)[#text(size: 20pt, fill: amber)[→]],
        align(center)[
          #text(size: 9pt, weight: "bold", fill: indigo)[Bottom-up ↑]
          #v(4pt)
          #box(inset: 3pt, fill: sky.lighten(80%), stroke: 0.5pt + sky, radius: 2pt)[#text(size: 8pt)[Строка 2]]
          #v(1pt)
          #box(inset: 3pt, fill: emerald.lighten(80%), stroke: 0.5pt + emerald, radius: 2pt)[#text(size: 8pt)[Строка 1]]
          #v(1pt)
          #box(inset: 3pt, fill: rose.lighten(80%), stroke: 0.5pt + rose, radius: 2pt)[#text(size: 8pt)[Строка 0]]
        ],
      )
    ]
  ],
  caption: [Каналы меняются местами + строки переворачиваются],
)

#joke[
  Инженеры IBM в 1985: «Давайте (0,0) будет внизу-слева — как в математике!» 📐 \
  Программисты в 2026: «...спасибо, очень помогло» 🤦
]

== 📊 PNG vs JPEG vs BMP

#figure(
  block(width: 100%, inset: 16pt, radius: 12pt, fill: violet.lighten(96%), stroke: 1.5pt + violet.lighten(55%))[
    #align(center)[
      #text(fill: violet, weight: "bold", size: 12pt)[📊 Сравнение форматов (Full HD)]
      #v(10pt)
      #grid(columns: (1fr, 1fr, 1fr), gutter: 12pt,
        colorbox(color: teal, title: "PNG 🖼️")[
          🔒 Без потерь \
          🎨 Есть альфа \
          📏 ~500 КБ \
          ⏱️ Медленнее \
          ✅ Для скриншотов
        ],
        colorbox(color: indigo, title: "JPEG 📸")[
          ⚡ С потерями \
          ❌ Нет альфы \
          📏 ~100 КБ \
          ⏱️ Быстрее \
          ✅ Для фото
        ],
        colorbox(color: amber, title: "BMP 📦")[
          📦 Без сжатия \
          🎨 Есть альфа \
          📏 ~8 МБ \
          ⏱️ Мгновенно \
          ⚠️ Огромный!
        ],
      )
    ]
  ],
  caption: [PNG — лучший выбор для скриншотов],
)

#pagebreak()

= 🧠 Оптимизация по памяти

#joke[
  C: «Я контролирую каждый байт!» \
  Rust: «Я тоже, но мне ещё и компилятор помогает 🦀» \
  Java: «А что такое байт?» ☕
]

#figure(
  table(
    columns: (auto, auto, 1fr),
    inset: 8pt,
    align: (left, center, left),
    fill: (col, row) => if row == 0 { violet.lighten(88%) } else if calc.odd(row) { violet.lighten(96%) } else { white },
    [*🔧 Тип*], [*📏 Размер*], [*📝 Примечание*],
    [`bool`], [1 байт], [Но `Option<bool>` тоже 1 байт! 🤯],
    [`enum Format`], [1 байт], [3 варианта помещаются в u8],
    [`u32`], [4 байта], [Размеры, FPS, индексы],
    [`&str`], [16 байт], [Указатель + длина (без аллокации!)],
    [`String`], [24 байта], [Указатель + длина + ёмкость (данные в куче)],
    [`Vec<u8>`], [24 байта], [Как String, только для байтов],
  ),
  caption: [Размеры типов — знание = сила 💪],
)

#funfact[
  `Option<&str>` занимает 16 байт — столько же, сколько `&str`! Rust использует нулевой указатель для `None`. Это называется *niche optimization* — бесплатная обёртка 🎁
]

#pagebreak()

= 🌍 Кроссплатформенность

#figure(
  block(width: 100%, inset: 16pt, radius: 12pt, fill: amber.lighten(96%), stroke: 1.5pt + amber.lighten(55%))[
    #align(center)[
      #text(fill: amber, weight: "bold", size: 12pt)[🔀 Что компилируется на каждой платформе]
      #v(10pt)
      #node("Общий код 🧠", color: emerald, icon: "")
      #v(8pt)
      #grid(columns: (1fr, 1fr, 1fr), gutter: 8pt,
        align(center)[
          #node("Windows", color: sky, icon: "🪟")
          #v(4pt)
          #text(size: 8pt)[GDI, COM, Registry \
          Media Foundation \
          Нативный диалог]
        ],
        align(center)[
          #node("Linux", color: amber, icon: "🐧")
          #v(4pt)
          #text(size: 8pt)[X11, XGetImage \
          xdg-open \
          Нет записи 😢]
        ],
        align(center)[
          #node("macOS", color: violet, icon: "🍎")
          #v(4pt)
          #text(size: 8pt)[Core Graphics \
          open command \
          Нет записи 😢]
        ],
      )
    ]
  ],
  caption: [Windows получает все фичи, остальные — основные],
)

#joke[
  *Windows*: «У меня есть COM, GDI, Media Foundation!» \
  *Linux*: «У меня есть X11... и xdg-open.» \
  *macOS*: «У меня есть красивый шрифт.» 🍎✨
]

== ⚠️ Типичные ловушки

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt, align: (left, left),
    fill: (col, row) => if row == 0 { rose.lighten(88%) } else if calc.odd(row) { rose.lighten(96%) } else { white },
    [*⚠️ Ловушка*], [*✅ Решение*],
    [Разделители путей 🔀], [`PathBuf::join()` вместо `"/dir/" + file`],
    [Кодировка 🔤], [`OsStr` для путей, `wide()` для Win32],
    [Перевод строки ↩️], [`\r\n` на Windows vs `\n` — `writeln!`],
    [DPI масштаб 🔍], [Всегда учитывайте scale factor!],
    [Event loop 🔄], [macOS: только из main thread!],
  ),
  caption: [Если что-то «работает на моей машине» — проверьте это 👆],
)

#pagebreak()

= 📜 Все принципы проекта

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt, align: (left, left),
    fill: (col, row) => if row == 0 { rust-orange.lighten(88%) } else if calc.odd(row) { rust-orange.lighten(96%) } else { white },
    [*📐 Принцип*], [*📝 Применение*],
    [🧱 SOLID — S], [Один модуль = одна ответственность],
    [🔓 SOLID — O], [`enum Format` расширяется без изменения кода],
    [🔌 SOLID — D], [Инъекция через замыкания],
    [🔄 DRY], [`generate_timestamp()` — один источник истины],
    [🎯 KISS], [Ручной JSON вместо serde для 5 полей],
    [🚫 YAGNI], [Не добавляем фичи «на будущее»],
    [🛡️ Безопасность], [`Result` + `?`, минимум unsafe],
    [🧪 TDD], [Тест → код → рефакторинг],
    [📋 `[+] / [-] / [!]`], [Конвенция коммитов],
  ),
  caption: [Философия проекта в одной таблице 🦀],
)

#colorbox(color: sky, title: "🚀 Что ещё можно внедрить")[
  - 🎲 *Fuzzing* — `cargo-fuzz` для `parse_settings_json`
  - 🧬 *Mutation testing* — `cargo-mutants`
  - 📊 *Coverage в CI* — `tarpaulin` + Codecov
  - 🏷️ *Feature flags* — `#[cfg(feature = "recording")]`
  - 📝 *Error enum* — `enum AppError` вместо `String`
]

#protip[
  Лучший код — тот, который не нужно писать. \
  Второй лучший — тот, который не нужно отлаживать. \
  Третий — тот, который покрыт тестами 🧪
]
