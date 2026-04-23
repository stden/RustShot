# Учебник по созданию инсталляторов для Rust

Это короткий практический учебник о том, как упаковать Rust-приложение для Windows в установщик, на примере `RustShot`.

## 1. Что делает Rust, а что делает установщик

Rust собирает ваш бинарник:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Установщик затем:

- копирует `.exe` в каталог установки
- создаёт ярлыки в меню «Пуск»
- при необходимости пишет ключи в реестр
- может запускать программу сразу после установки
- добавляет деинсталлятор

В `RustShot` Rust собирает `rustshot.exe`, а Inno Setup делает из него `RustShotSetup.exe`.

## 2. Почему в проекте выбран Inno Setup

В `RustShot` используется [installer.iss](../installer.iss) и Inno Setup, потому что для небольшого Windows desktop utility это практичный выбор:

- простой формат скрипта
- легко настраивать ярлыки и реестр
- удобно делать silent install
- не нужно писать сам установщик на Rust

Если вам нужен именно MSI, чаще лучше подходит WiX. Если нужен обычный `.exe`-установщик, Inno Setup обычно проще.

## 3. Какие файлы отвечают за установщик в RustShot

Основные файлы такие:

- [installer.iss](../installer.iss) — описание установщика
- [.github/workflows/release.yml](../.github/workflows/release.yml) — CI/CD сборка релиза и инсталлятора
- [README.ru.md](../README.ru.md) — что скачивает пользователь

## 4. Минимальный локальный workflow

Установить Rust target:

```powershell
rustup target add x86_64-pc-windows-msvc
```

Собрать release binary:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Подготовить путь, который ожидает Inno Setup:

```powershell
New-Item -ItemType Directory -Force -Path target\release | Out-Null
Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe
```

Собрать установщик:

```powershell
iscc installer.iss
```

Результат:

- portable binary: `target\x86_64-pc-windows-msvc\release\rustshot.exe`
- installer: `Output\RustShotSetup.exe`

## 5. Из чего состоит `installer.iss`

### `[Setup]`

Здесь задаются метаданные и общие настройки:

- `AppName`
- `AppVersion`
- `DefaultDirName`
- `OutputBaseFilename`
- `PrivilegesRequired`

В `RustShot` параметр `PrivilegesRequired=lowest` важен, потому что программа работает в пользовательском контексте и пишет autorun в `HKCU`, а не в `HKLM`.

### `[Files]`

Секция перечисляет устанавливаемые файлы:

```iss
[Files]
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
```

Для более сложного приложения сюда ещё могут входить:

- иконки
- шаблоны конфигов
- дополнительные DLL
- license/readme файлы

### `[Icons]`

Создаёт ярлыки:

```iss
[Icons]
Name: "{group}\RustShot"; Filename: "{app}\rustshot.exe"
Name: "{group}\Uninstall RustShot"; Filename: "{uninstallexe}"
```

### `[Registry]`

Создаёт записи в реестре:

```iss
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "RustShot"; ValueData: """{app}\rustshot.exe"""; Flags: uninsdeletevalue
```

Именно так `RustShot` добавляет автозапуск tray-приложения после логина.

### `[Run]`

Позволяет запустить программу сразу после установки:

```iss
[Run]
Filename: "{app}\rustshot.exe"; Description: "Launch RustShot (system tray)"; Flags: nowait postinstall skipifsilent
```

## 6. Минимальный шаблон для другого Rust-приложения

```iss
[Setup]
AppName=MyRustApp
AppVersion=0.1.0
DefaultDirName={autopf}\MyRustApp
DefaultGroupName=MyRustApp
OutputDir=Output
OutputBaseFilename=MyRustAppSetup
Compression=lzma
SolidCompression=yes
PrivilegesRequired=lowest
WizardStyle=modern

[Files]
Source: "target\release\my_rust_app.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\MyRustApp"; Filename: "{app}\my_rust_app.exe"
Name: "{group}\Uninstall MyRustApp"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\my_rust_app.exe"; Description: "Launch MyRustApp"; Flags: nowait postinstall skipifsilent
```

Для утилит и небольших desktop apps этого часто уже достаточно.

## 7. Как развивать установщик дальше

Обычно следующий шаг такой:

- добавить иконку приложения
- добавить `AppPublisher`, `AppSupportURL`, `AppPublisherURL`
- добавить file associations
- сделать desktop shortcut как опциональную задачу
- выбрать per-user или per-machine установку
- подписывать и `.exe`, и сам установщик

## 8. Как это собрано в CI

Workflow в [release.yml](../.github/workflows/release.yml):

1. checkout репозитория
2. установка MSVC target
3. сборка Rust binary
4. копирование `.exe` в `target\release`
5. запуск `iscc installer.iss`
6. публикация portable `.exe` и `RustShotSetup.exe`

Это удобно, потому что пользователям часто нужны оба варианта:

- portable executable
- полноценный установщик

## 9. Как правильно тестировать установщик

Недостаточно проверить только факт сборки.

Нужно проверить:

1. Чистую установку на новом профиле Windows.
2. Запуск из меню «Пуск».
3. Корректность директорий и файлов после установки.
4. Правильное создание autorun в реестре.
5. Silent install.
6. Деинсталляцию.
7. Обновление поверх старой версии.

Для `RustShot` отдельно важно проверить:

- tray запускается после установки
- запись в `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` корректна
- uninstall удаляет эту запись

## 10. Типичные ошибки

- Собрать GNU target, а упаковывать как будто это MSVC build.
- Указать в `[Files]` не тот путь до `.exe`.
- Требовать admin install, хотя приложение спроектировано как per-user.
- Не тестировать uninstall.
- Думать, что `cargo build --release` сам по себе создаёт установщик.
- Держать версию в нескольких местах и позволять ей расходиться.

## 11. Практическое улучшение для RustShot

Сейчас `AppVersion=0.1.0` захардкожен в [installer.iss](../installer.iss). Практичный следующий шаг — подставлять эту версию из `Cargo.toml` или из CI, чтобы версия crate и версия инсталлятора не расходились.

## 12. Следующий урок

Если нужен более подробный разбор именно `installer.iss` и GitHub Actions release flow, смотрите [INNO_SETUP_PIPELINE.ru.md](INNO_SETUP_PIPELINE.ru.md).
