# Урок по Inno Setup и release pipeline

Этот урок разбирает две конкретные части релизного процесса `RustShot`:

1. как собирается Windows-установщик через Inno Setup
2. как GitHub Actions release pipeline превращает тег в готовые артефакты

Это не абстрактная статья, а объяснение на материале текущего репозитория.

## 1. Какие артефакты выпускает проект

`RustShot` публикует два Windows-артефакта:

- portable binary: `rustshot.exe`
- installer: `RustShotSetup.exe`

Это видно в [README.ru.md](../README.ru.md), [installer.iss](../installer.iss) и [.github/workflows/release.yml](../.github/workflows/release.yml).

## 2. За что отвечает Inno Setup

Inno Setup не компилирует Rust-код.

Его зона ответственности начинается после того, как `.exe` уже собран. Он отвечает за:

- копирование бинарника в каталог установки
- создание ярлыков в меню «Пуск»
- добавление записей в реестр для автозапуска
- создание деинсталлятора
- при необходимости запуск приложения после установки
- упаковку всего этого в один installer executable

В этом проекте такая логика описана в [installer.iss](../installer.iss).

## 3. Как читать `installer.iss`

### `[Setup]`

Это шапка установщика и его глобальные настройки:

- `AppName=RustShot`
- `AppVersion=0.1.0`
- `DefaultDirName={autopf}\RustShot`
- `OutputDir=Output`
- `OutputBaseFilename=RustShotSetup`
- `PrivilegesRequired=lowest`

Ключевое решение здесь — `PrivilegesRequired=lowest`.

Это означает:

- установка идёт в пользовательском контексте
- не нужен admin prompt
- приложение может корректно прописать автозапуск в `HKCU`

Для `RustShot` это соответствует его tray-модели и per-user startup.

### `[Languages]`

Интерфейс установщика доступен на:

- английском
- русском

Это логично, потому что документация проекта тоже двуязычная.

### `[Files]`

Сейчас установщик кладёт один основной payload:

```iss
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
```

Именно поэтому release workflow сначала копирует MSVC build в `target\release`, а уже потом запускает `iscc`.

### `[Icons]`

Эта секция создаёт ярлыки:

- запуск `RustShot`
- запуск деинсталлятора

### `[Registry]`

Установщик добавляет запись:

`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\RustShot`

Именно она запускает tray-приложение после логина.

Флаг `uninsdeletevalue` важен, потому что удаляет эту запись при uninstall.

### `[Run]`

После установки программа может запускаться автоматически:

- это удобно для tray app
- в silent install это пропускается из-за `skipifsilent`

## 4. Локальный flow сборки установщика

Локальный flow такой:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
New-Item -ItemType Directory -Force -Path target\release | Out-Null
Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe
iscc installer.iss
```

Важная деталь:

`installer.iss` ожидает `target\release\rustshot.exe`, а Rust build кладёт файл в `target\x86_64-pc-windows-msvc\release\rustshot.exe`.

Это нормализуется вручную локально и тем же образом в CI.

## 5. Как читать release pipeline

Релизный workflow лежит в [release.yml](../.github/workflows/release.yml).

Он запускается по:

- тегам вида `v*`
- ручному `workflow_dispatch`

Основные шаги такие:

1. checkout исходников
2. установка stable Rust и target `x86_64-pc-windows-msvc`
3. восстановление Cargo cache
4. сборка release binary
5. копирование бинарника в `target\release`
6. запуск `iscc installer.iss`
7. загрузка portable `.exe` как artifact
8. загрузка installer как artifact
9. если это tag build, публикация обоих файлов в GitHub Releases

## 6. Почему pipeline использует MSVC target

Workflow явно собирает:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Это важно, потому что:

- MSVC — стандартный target для shipping Windows desktop apps
- смешивание GNU и MSVC output легко ломает packaging
- CI стандартизирован вокруг одного ожидаемого output path

Если локально разработчик собирает другой target, важно не перепутать бинарник при упаковке.

## 7. Поток артефактов

Поток данных выглядит так:

```text
Cargo build
  -> target\x86_64-pc-windows-msvc\release\rustshot.exe
  -> copy to target\release\rustshot.exe
  -> consume from installer.iss
  -> Output\RustShotSetup.exe
  -> upload both files as GitHub Actions artifacts
  -> attach both files to GitHub Release
```

То есть у проекта есть два разных “финальных” файла:

- прямой бинарник из Cargo
- установщик, собранный Inno Setup

## 8. Типичные точки отказа

### Неверный путь в `[Files]`

Если `installer.iss` смотрит не туда, `iscc` упадёт, даже если Rust-код собрался нормально.

### Расхождение версии

`Cargo.toml` и `installer.iss` могут содержать разные версии.

В этом репозитории это реальный риск, потому что `AppVersion` пока захардкожен.

### Артефакты собраны, но релиз не опубликован

Во время `workflow_dispatch` workflow может собрать артефакты, но GitHub Release создаётся только для tag refs.

### Неожиданности в silent install

То, что нормально ведёт себя в обычной установке, может вести себя иначе в silent install.

### Конфликт per-user и per-machine

Если позже переключиться на admin install, но оставить per-user предположения в приложении и реестре, поведение станет непоследовательным.

## 9. Как дебажить проблемы инсталлятора

Полезный порядок такой:

1. Проверить, собрался ли Rust binary.
2. Проверить, скопировался ли `.exe` в `target\release`.
3. Проверить, проходит ли локально `iscc installer.iss`.
4. Проверить, содержит ли установщик нужные файлы и ярлыки.
5. Проверить, прикрепились ли оба артефакта к GitHub Release.

Этот порядок важен, потому что разделяет Rust build failures и packaging failures.

## 10. Что можно улучшить дальше

Практичные следующие шаги для `RustShot`:

- подставлять `AppVersion` из `Cargo.toml` или из тега
- добавить code signing для `rustshot.exe` и `RustShotSetup.exe`
- добавить installer smoke tests в CI
- проверять поведение autorun после install
- тестировать upgrade path со старых версий
- публиковать checksums рядом с release artifacts

## 11. Практическое правило

Считайте installer и release pipeline частью продуктового кода.

Если приложение работает, но install, autorun, upgrade или uninstall сломаны, значит релиз всё равно сломан.
