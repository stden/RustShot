# 💎 GEMINI.md

Это инструкция для Gemini при работе с `RustShot`.

## 📸 Коротко о проекте

`RustShot` - это программа для скриншотов и записи экрана.

Главные файлы:

- `src/lib.rs`
- `src/main.rs`
- `src/cli.rs`
- `src/capture.rs`
- `src/selection.rs`
- `src/tray.rs`
- `src/record.rs`
- `src/settings.rs`

## 🌍 Language

All descriptions, comments, commit messages, code reviews, and development discussions must be in English.

## ✅ Простые правила

- Не усложняй решение без необходимости.
- Не меняй поведение программы просто так.
- Старайся писать код так, чтобы его было легко тестировать.
- Если меняешь поведение, добавь тест.

## ⚡ Полезные команды

```bash
cargo build
cargo check
cargo test
cargo fmt
cargo clippy
cargo bench
python wiki/lint.py
```

## 📐 Как лучше писать изменения

Лучший порядок работы:

1. ✏️ Написать тест.
2. ❌ Получить падающий тест.
3. ✅ Исправить код.
4. ✅ Убедиться, что тест проходит.
5. 🧪 Запустить `cargo test`.

## 🎯 На что здесь лучше делать упор

- На маленькие helper-функции
- На понятные сообщения об ошибках
- На тесты для CLI, capture, selection и tray-логики

## 🚫 Чего лучше избегать

- Больших ненужных рефакторингов
- Новых зависимостей без пользы
- Сложных GUI-тестов, если можно проверить логику проще

## 📝 Если меняешь документацию

- Проверь, что примеры в `README.md` совпадают с реальным поведением программы.
- Не хардкодь динамические числа (например, количество тестов) в документации и избегай лишних правок документации.
- Если поменялись правила разработки, обнови и этот файл тоже.

## 🧠 Retrieval knowledge base

A persistent knowledge base for RustShot, located in `wiki/`.

### Structure
- Canonical sources stay in code, `articles/`, and `docs/`; the corpus does not duplicate raw files.
- `wiki/pages/`: Retrieval chunks with stable IDs, tags, canonical source paths, and related-page IDs.
- `wiki/index.md`: Task-oriented catalog of every corpus page.
- `wiki/templates/`: Metadata and section contracts for concepts, entities, and source summaries.

### Operations
- **Ingest**: Start from a template, summarize only RustShot-specific facts, list every canonical source in `source_paths`, and add the page once to `index.md`.
- **Query**: Match the question to index descriptions and page tags, read the relevant chunks, then verify claims in their canonical sources.
- **Lint**: Run `python wiki/lint.py`; CI rejects malformed metadata, duplicate IDs, missing sources, broken links, invalid relationships, and unindexed pages.
