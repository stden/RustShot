# Лучшие практики Rust: код и производительность

Учебник на примерах из проекта RustShot.

## Оглавление

1. [Структура проекта](#1-структура-проекта)
2. [Владение и заимствование](#2-владение-и-заимствование)
3. [Обработка ошибок](#3-обработка-ошибок)
4. [Производительность](#4-производительность)
5. [Unsafe Rust](#5-unsafe-rust)
6. [Тестирование](#6-тестирование)
7. [Бенчмарки](#7-бенчмарки)

---

## 1. Структура проекта

### Разделяй библиотеку и бинарник

```
src/
  lib.rs       # Публичный API для тестов и бенчмарков
  main.rs      # Точка входа, только CLI
  capture.rs   # Логика скриншотов
  record.rs    # Запись видео
  selection.rs # Выбор области
  settings.rs  # Настройки
  tray.rs      # Системный трей
```

`lib.rs` реэкспортирует модули:

```rust
pub mod capture;
pub mod record;
pub mod selection;
pub mod settings;
pub mod tray;
```

Зачем: бенчмарки (`benches/`) и интеграционные тесты (`tests/`) могут подключать
`use rustshot::capture` только если модуль публичный через `lib.rs`.

### Cargo.toml — минимум зависимостей

```toml
[dependencies]
image = "0.25"        # Обработка изображений
clap = { version = "4", features = ["derive"] }  # CLI
chrono = "0.4"        # Временные метки
dirs = "6"            # Стандартные пути ОС
```

Правило: каждая зависимость — это вес бинарника, время компиляции, потенциальные уязвимости.
Если можно написать 20 строк кода вместо добавления крейта — пиши код.

Пример: `settings.rs` сериализует JSON вручную вместо `serde_json`:

```rust
pub fn format_settings_json(settings: &Settings) -> String {
    let screenshots = escape_json_string(&settings.screenshots_dir.display().to_string());
    let mut buf = String::with_capacity(256);
    buf.push_str("{\n  \"screenshots_dir\": \"");
    buf.push_str(&screenshots);
    // ...
    buf
}
```

---

## 2. Владение и заимствование

### Избегай клонирования в горячих путях

```rust
// Плохо: клонирует весь буфер каждый раз
fn process(data: Vec<u8>) -> Vec<u8> { ... }

// Хорошо: заимствует, не копирует
fn process(data: &[u8]) -> Vec<u8> { ... }

// Ещё лучше: модифицирует на месте
fn process(data: &mut [u8]) { ... }
```

Пример из RustShot — `bgra_to_rgba` работает in-place:

```rust
pub fn bgra_to_rgba(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2); // Меняем B и R местами
    }
}
```

### Используй слайсы вместо Vec

```rust
// Плохо: требует аллокации
fn default_screen_index(flags: Vec<bool>) -> usize

// Хорошо: принимает любую коллекцию
fn default_screen_index(flags: &[bool]) -> usize
```

Исключение: если функция вызывается редко и слайс создаётся рядом, `Vec` в параметре допустим
для простоты API.

---

## 3. Обработка ошибок

### `Result<T, String>` для простых проектов

```rust
pub fn resolve_screen_index(
    screen_count: usize,
    requested: Option<usize>,
) -> Result<usize, String> {
    match requested {
        None => Ok(0),
        Some(i) if i < screen_count => Ok(i),
        Some(i) => Err(format!(
            "Screen index {} out of range (0..{})", i, screen_count
        )),
    }
}
```

Для больших проектов используй `thiserror` или кастомный enum ошибок.

### Понятные сообщения об ошибках

```rust
// Плохо
Err("invalid".to_string())

// Хорошо
Err(format!("FPS must be > 0 and <= 120, got {}", fps))
```

Правило: сообщение должно содержать:
- Что ожидалось
- Что получено
- Контекст (если не очевидно)

---

## 4. Производительность

### 4.1. Предвыделяй память

```rust
// Плохо: String растёт и реаллоцируется 3-5 раз
let mut buf = String::new();

// Хорошо: одна аллокация
let mut buf = String::with_capacity(256);
```

Результат в RustShot: `format_settings_json` стал на **24% быстрее** (265ns → 211ns).

### 4.2. Целочисленная арифметика вместо float

```rust
// Медленно: sqrt для каждого пикселя
let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
if dist <= 6.0 { ... }

// Быстро: сравниваем квадраты
let dist_sq = dx * dx + dy * dy;
if dist_sq <= 36 { ... }  // 6² = 36
```

Пример из `tray.rs` — иконка 32x32 = 1024 пикселя. Убрали 1024 вызова `sqrt()`.

### 4.3. Компилятор умнее тебя (обычно)

Rust с `-O2` / `--release` автовекторизует простые циклы. Пример:

```rust
// Простой вариант — компилятор генерирует SIMD
for pixel in data.chunks_exact_mut(4) {
    pixel.swap(0, 2);
}

// "Оптимизированный" unsafe вариант — МЕДЛЕННЕЕ!
unsafe {
    let ptr = data.as_mut_ptr();
    for i in 0..data.len() / 4 {
        std::ptr::swap(ptr.add(i * 4), ptr.add(i * 4 + 2));
    }
}
```

В RustShot unsafe `bgra_to_rgba` оказался **в 2.5 раза медленнее** (2.72ms → 6.94ms на 1M пикселей).
Причина: `chunks_exact_mut` даёт компилятору гарантию выровненности и размера, что позволяет
генерировать SIMD-инструкции. Ручной `ptr::swap` убирает эти гарантии.

**Правило: измеряй, прежде чем "оптимизировать".**

### 4.4. Двухпроходная обработка

Иногда разделение операций на два прохода быстрее, чем один сложный проход:

```rust
pub fn rgba_to_bgra_bottom_up(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_bytes = (width * 4) as usize;
    let total = (width * height * 4) as usize;
    let mut out = vec![0u8; total];

    // Проход 1: копируем строки в обратном порядке (memcpy)
    for y in 0..height as usize {
        let src_start = y * row_bytes;
        let dst_start = (height as usize - 1 - y) * row_bytes;
        out[dst_start..dst_start + row_bytes]
            .copy_from_slice(&rgba[src_start..src_start + row_bytes]);
    }

    // Проход 2: меняем R и B каналы на месте
    let ptr = out.as_mut_ptr();
    let pixel_count = total / 4;
    for i in 0..pixel_count {
        unsafe {
            let base = ptr.add(i * 4);
            std::ptr::swap(base, base.add(2));
        }
    }

    out
}
```

Почему быстрее: `copy_from_slice` компилируется в `memcpy`, который использует AVX/SSE.
Второй проход тоже может быть векторизован, т.к. данные уже в линейном порядке.

### 4.5. Раннее вычисление инвариантов

```rust
// Плохо: пересчитываем каждую итерацию
for y in 0..size {
    for x in 0..size {
        let dy = (y as i32 - 16).abs();
        let in_body_y = dy <= 10;
        // ...
    }
}

// Хорошо: выносим из внутреннего цикла
for y in 0..size {
    let dist_y = (y as i32 - 16).abs();
    let in_body_y = dist_y <= 10;
    let dist_y_sq = dist_y * dist_y;

    for x in 0..size {
        // dist_y, in_body_y, dist_y_sq уже посчитаны
    }
}
```

### 4.6. Ноль CPU в idle

Если приложение живёт в системном трее:

```rust
// winit event loop с ControlFlow::Wait
// Поток спит, пока нет событий = 0% CPU
event_loop.run(|event, target| {
    target.set_control_flow(ControlFlow::Wait);
    // ...
});
```

---

## 5. Unsafe Rust

### Когда unsafe оправдан

1. **FFI** — вызов Windows API через крейт `windows`
2. **Производительность** — только если бенчмарки доказали выигрыш
3. **Интроп** — работа с raw-указателями из C-библиотек

### Когда unsafe НЕ оправдан

- Для "оптимизации" простых циклов (компилятор обычно лучше)
- Для обхода borrow checker (это баг в дизайне, не в компиляторе)
- Если safe-версия читабельнее и не медленнее

### Правила unsafe

```rust
// Всегда документируй инвариант
let ptr = out.as_mut_ptr();
for i in 0..pixel_count {
    unsafe {
        // SAFETY: i * 4 + 2 < total, т.к. pixel_count = total / 4
        let base = ptr.add(i * 4);
        std::ptr::swap(base, base.add(2));
    }
}
```

---

## 6. Тестирование

### Тестируй граничные случаи

```rust
#[test]
fn resolve_screen_index_out_of_range() {
    assert!(capture::resolve_screen_index(3, Some(5)).is_err());
}

#[test]
fn resolve_screen_index_exactly_at_boundary() {
    // Индекс 2 при 3 мониторах — последний валидный
    assert_eq!(capture::resolve_screen_index(3, Some(2)).unwrap(), 2);
}

#[test]
fn resolve_screen_index_one_past_boundary() {
    // Индекс 3 при 3 мониторах — первый невалидный
    assert!(capture::resolve_screen_index(3, Some(3)).is_err());
}
```

### Тестируй текст ошибок

```rust
#[test]
fn error_message_contains_actual_value() {
    let err = validate_recording_start(0).unwrap_err();
    assert!(err.contains("0"), "Error should mention the invalid value");
}
```

### Regression-тесты

Нашёл баг → напиши тест, который его воспроизводит → исправь код:

```rust
#[test]
fn logical_to_physical_half_rounds_correctly() {
    // Раньше ожидали 4, но f32::round() округляет 4.5 → 5
    assert_eq!(logical_to_physical(3, 1.5), 5);
}
```

### Именование тестов

```
module_function_scenario
```

Примеры:
- `bgra_to_rgba_empty_buffer`
- `resolve_screen_index_out_of_range`
- `build_region_from_drag_too_small`

---

## 7. Бенчмарки

### Criterion — стандарт для Rust

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "benchmarks"
harness = false
```

### Структура бенчмарка

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_bgra_to_rgba(c: &mut Criterion) {
    let mut group = c.benchmark_group("bgra_to_rgba");

    // Маленький вход — показывает overhead
    group.bench_function("4px", |b| {
        let src: Vec<u8> = (0..16).collect();
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    // Большой вход — показывает throughput
    group.bench_function("1M_px", |b| {
        let src: Vec<u8> = (0..4_194_304).map(|i| (i % 256) as u8).collect();
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    group.finish();
}
```

### `black_box` — обязателен

```rust
// Без black_box компилятор может выкинуть вычисление
b.iter(|| some_function(black_box(input)));
```

### Запуск

```bash
cargo bench                    # Все бенчмарки
cargo bench -- bgra_to_rgba    # Только конкретная группа
```

### Workflow оптимизации

1. Напиши бенчмарк **до** оптимизации
2. Запусти `cargo bench` — получи baseline
3. Сделай изменение
4. Запусти `cargo bench` — сравни результаты
5. Если медленнее — **откати**
6. Если быстрее — зафиксируй числа в коммите

---

## Итого: чеклист

- [ ] Минимум зависимостей
- [ ] `&[T]` вместо `Vec<T>` в параметрах
- [ ] `with_capacity()` для `String` и `Vec`
- [ ] Целочисленная арифметика где можно
- [ ] Бенчмарк перед и после оптимизации
- [ ] Unsafe только с доказанным выигрышем
- [ ] Тесты на граничные случаи
- [ ] Понятные сообщения об ошибках
- [ ] `cargo fmt && cargo clippy` перед коммитом
