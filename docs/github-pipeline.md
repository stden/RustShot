# GitHub Actions: CI/CD pipeline для Rust

Учебник на примерах из проекта RustShot.

## Оглавление

1. [Что такое CI/CD](#1-что-такое-cicd)
2. [Структура workflow](#2-структура-workflow)
3. [CI pipeline — проверка кода](#3-ci-pipeline--проверка-кода)
4. [Release pipeline — сборка и публикация](#4-release-pipeline--сборка-и-публикация)
5. [Кэширование](#5-кэширование)
6. [Кросс-платформенная сборка](#6-кросс-платформенная-сборка)
7. [Типичные проблемы](#7-типичные-проблемы)

---

## 1. Что такое CI/CD

**CI (Continuous Integration)** — автоматическая проверка каждого коммита:
- Код компилируется?
- Тесты проходят?
- Форматирование правильное?
- Линтер не ругается?

**CD (Continuous Delivery)** — автоматическая сборка и публикация релизов.

Файлы живут в `.github/workflows/`:

```
.github/
  workflows/
    ci.yml       # Проверка на каждый push/PR
    release.yml  # Сборка при создании тега
```

---

## 2. Структура workflow

```yaml
name: CI                    # Имя, видно в GitHub UI

on:                         # Когда запускать
  push:
    branches: ["main"]      # На push в main
  pull_request:             # На любой PR

env:                        # Переменные окружения
  CARGO_TERM_COLOR: always

jobs:                       # Список задач
  fmt:                      # Имя задачи (произвольное)
    name: Format            # Отображаемое имя
    runs-on: ubuntu-latest  # ОС раннера
    steps:                  # Шаги
      - uses: actions/checkout@v4
      - run: cargo fmt --all --check
```

### Ключевые поля

| Поле | Описание |
|------|----------|
| `on.push.branches` | На какие ветки реагировать |
| `on.push.tags` | На какие теги реагировать |
| `on.pull_request` | Запуск на PR |
| `on.workflow_dispatch` | Ручной запуск через UI |
| `runs-on` | ОС: `ubuntu-latest`, `windows-latest`, `macos-latest` |
| `steps` | Последовательные шаги |
| `uses` | Готовый action из marketplace |
| `run` | Команда bash/powershell |

---

## 3. CI pipeline — проверка кода

Полный CI из RustShot с пояснениями:

### Job 1: Форматирование

```yaml
fmt:
  name: Format
  runs-on: ubuntu-latest   # Формат не зависит от ОС — берём самый быстрый
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt
    - run: cargo fmt --all --check
```

`cargo fmt --check` не форматирует, а только проверяет. Если код не отформатирован — exit code 1.

### Job 2: Линтер

```yaml
clippy:
  name: Clippy
  runs-on: windows-latest  # Windows — основная платформа проекта
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - run: cargo clippy --all-targets -- -D warnings
```

`-D warnings` — превращает предупреждения clippy в ошибки. Без этого флага
clippy может предупреждать, но не ронять билд.

`--all-targets` — проверяет и тесты, и бенчмарки, и примеры.

### Job 3: Тесты Windows

```yaml
test-windows:
  name: Test (Windows)
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

### Job 4: Тесты Linux

```yaml
test-linux:
  name: Test (Linux)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install X11 and GTK dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y \
          libxcb1-dev \
          libxcb-render0-dev \
          libxcb-shape0-dev \
          libxcb-xfixes0-dev \
          libglib2.0-dev \
          libgtk-3-dev \
          libxdo-dev \
          libatk1.0-dev \
          libgdk-pixbuf2.0-dev \
          libcairo2-dev \
          libpango1.0-dev

    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

На Linux нужны системные библиотеки для GUI-зависимостей. Без них `cargo test` упадёт
на этапе линковки.

### Параллельность

Все 4 job'а запускаются **параллельно** по умолчанию. Если нужна зависимость:

```yaml
test-windows:
  needs: [fmt, clippy]  # Запустится только после fmt и clippy
```

В RustShot зависимости нет — все jobs независимы для скорости.

---

## 4. Release pipeline — сборка и публикация

### Триггер — тег с версией

```yaml
on:
  push:
    tags:
      - 'v*'           # v0.1.0, v1.0.0, etc.
  workflow_dispatch:    # + ручной запуск
```

### Создание релиза

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Полный release workflow

```yaml
jobs:
  build-windows:
    name: Build Windows Release
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      # Кэш ускоряет повторные сборки
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      # Сборка release-бинарника
      - run: cargo build --release --target x86_64-pc-windows-msvc

      # Копируем бинарник для инсталлятора
      - name: Prepare installer source
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force -Path target\release | Out-Null
          Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe

      # Собираем инсталлятор (Inno Setup предустановлен на windows-latest)
      - run: iscc installer.iss

      # Загружаем артефакты
      - uses: actions/upload-artifact@v4
        with:
          name: rustshot-windows-x64-portable
          path: target\x86_64-pc-windows-msvc\release\rustshot.exe

      - uses: actions/upload-artifact@v4
        with:
          name: rustshot-windows-x64-installer
          path: Output\RustShotSetup.exe

      # Создаём GitHub Release с файлами
      - name: Create GitHub Release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            target/x86_64-pc-windows-msvc/release/rustshot.exe
            Output/RustShotSetup.exe
          generate_release_notes: true
```

### Артефакты

`upload-artifact` сохраняет файлы, которые можно скачать из UI GitHub Actions:
- `rustshot-windows-x64-portable` — просто .exe
- `rustshot-windows-x64-installer` — инсталлятор с Inno Setup

### GitHub Release

`softprops/action-gh-release@v2` создаёт релиз автоматически при push тега:
- Прикрепляет указанные файлы
- `generate_release_notes: true` — автоматически генерирует changelog из коммитов

---

## 5. Кэширование

### Зачем кэш

Без кэша каждый билд скачивает и компилирует все зависимости с нуля (~2-5 минут).
С кэшем — только дельту.

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry    # Скачанные исходники крейтов
      ~/.cargo/git         # Git-зависимости
      target               # Скомпилированные зависимости
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

`key` включает хэш `Cargo.lock` — если зависимости не изменились, кэш валиден.

### Когда НЕ кэшировать

В CI (не release) кэш часто не стоит усилий:
- Параллельные jobs всё равно быстрые
- Кэш может скрыть проблемы с зависимостями
- Инвалидация кэша — источник трудноуловимых багов

---

## 6. Кросс-платформенная сборка

### Матричная стратегия (пример)

```yaml
test:
  strategy:
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

### Условные шаги

```yaml
- name: Install Linux deps
  if: runner.os == 'Linux'
  run: sudo apt-get install -y libxcb1-dev ...
```

### Условная компиляция в Rust

```yaml
# В CI — все платформы, в коде — cfg атрибуты:
```

```rust
#[cfg(target_os = "windows")]
fn register_hotkey() { ... }

#[cfg(not(target_os = "windows"))]
fn register_hotkey() {
    // no-op на Linux/macOS
}
```

---

## 7. Типичные проблемы

### Форматирование

**Проблема**: CI падает на `cargo fmt --check`, хотя локально всё ок.

**Решение**: разные версии rustfmt. Зафиксируй toolchain:

```yaml
- uses: dtolnay/rust-toolchain@stable
```

Или конкретная версия:

```yaml
- uses: dtolnay/rust-toolchain@1.77.0
```

И запускай `cargo fmt` перед коммитом.

### Системные зависимости на Linux

**Проблема**: `cargo test` падает с `cannot find -lxcb`.

**Решение**: установи dev-пакеты:

```yaml
- run: |
    sudo apt-get update
    sudo apt-get install -y libxcb1-dev libgtk-3-dev ...
```

Как найти нужные пакеты: читай ошибку линковщика, ищи `apt-file search libXXX.so`.

### Windows и Inno Setup

**Проблема**: `iscc` не найден.

**Решение**: `iscc` предустановлен на `windows-latest` GitHub runner. Если нужна
конкретная версия:

```yaml
- uses: jrsoftware/iscc-action@v1
```

### Секреты

```yaml
# Никогда не хардкодь токены в workflow!
- run: something --token ${{ secrets.MY_TOKEN }}
```

Секреты настраиваются в Settings → Secrets and variables → Actions.

---

## Чеклист нового workflow

- [ ] `on:` — правильные триггеры
- [ ] `runs-on:` — правильная ОС
- [ ] `dtolnay/rust-toolchain` — для Rust проектов
- [ ] Системные зависимости для Linux
- [ ] `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`
- [ ] Кэш для release builds
- [ ] `upload-artifact` для билд-артефактов
- [ ] Секреты через `${{ secrets.XXX }}`
