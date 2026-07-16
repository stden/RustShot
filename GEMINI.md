# 💎 GEMINI.md

Это инструкция для Gemini при работе с `RustShot`.

## 📸 Коротко о проекте

`RustShot` - это программа для скриншотов и записи экрана.

Главные файлы:

- `src/main.rs`
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
- Если поменялись правила разработки, обнови и этот файл тоже.

## 🧠 LLM Wiki

A persistent knowledge base for RustShot. Located in `wiki/`.

### Structure
- Raw source documents live in their canonical homes: `articles/` (long-form
  RU essays) and `docs/` (technical/pipeline docs). The wiki no longer keeps a
  duplicate `wiki/raw/` copy; each `wiki/pages/source-*.md` links to its origin.
- `wiki/pages/`: Synthesized wiki pages (concepts, entities, summaries).
- `wiki/index.md`: Content-oriented catalog of all pages.
- `wiki/log.md`: Chronological log of wiki operations.

### Operations
- **Ingest**: Process a source file from `articles/` or `docs/`. Read, discuss, summarize in `wiki/pages/`, update `index.md` and `log.md`.
- **Query**: Search `index.md`, read relevant `pages/`, synthesize answer, optionally save as a new page.
- **Lint**: Check for contradictions, stale claims, orphan pages, and missing cross-references.
