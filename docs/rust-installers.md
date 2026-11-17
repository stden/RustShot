# Создание инсталляторов для программ на Rust

Учебник на примерах из проекта RustShot.

## Оглавление

1. [Способы распространения](#1-способы-распространения)
2. [Inno Setup — Windows инсталлятор](#2-inno-setup--windows-инсталлятор)
3. [MSI через WiX](#3-msi-через-wix)
4. [cargo-wix](#4-cargo-wix)
5. [NSIS](#5-nsis)
6. [Портативный .exe](#6-портативный-exe)
7. [Автозапуск и трей](#7-автозапуск-и-трей)
8. [Сборка в GitHub Actions](#8-сборка-в-github-actions)
9. [Linux: deb и AppImage](#9-linux-deb-и-appimage)
10. [Подпись кода](#10-подпись-кода)

---

## 1. Способы распространения

| Способ | Плюсы | Минусы |
|--------|-------|--------|
| Портативный .exe | Простота, zero install | Нет деинсталляции, нет автозапуска |
| Inno Setup (.exe) | Привычный wizard, автозапуск, деинсталляция | Только Windows |
| MSI (WiX) | Корпоративный стандарт, GPO | Сложная настройка |
| NSIS (.exe) | Гибкий скриптинг | Устаревший синтаксис |
| cargo-install | Для разработчиков | Нужен Rust toolchain |
| winget/scoop/choco | Менеджер пакетов | Нужна публикация в репозиторий |

Для RustShot выбран **Inno Setup** — баланс простоты и функциональности.

---

## 2. Inno Setup — Windows инсталлятор

### Что это

Бесплатный инсталлятор-генератор. Берёт ваш .exe и создаёт красивый setup wizard
с деинсталляцией, ярлыками, записью в реестр.

### Установка

- Скачай с https://jrsoftware.org/isinfo.php
- На GitHub Actions `windows-latest` уже предустановлен

### Файл installer.iss

Полный пример из RustShot:

```iss
[Setup]
AppName=RustShot
AppVersion=0.1.0
AppPublisher=RustShot Contributors
AppPublisherURL=https://github.com/stden/RustShot
AppSupportURL=https://github.com/stden/RustShot/issues
DefaultDirName={autopf}\RustShot
DefaultGroupName=RustShot
OutputDir=Output
OutputBaseFilename=RustShotSetup
Compression=lzma
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
```

### Разбор секций

#### [Setup] — основные настройки

```iss
AppName=RustShot           ; Имя в Programs and Features
AppVersion=0.1.0           ; Версия
DefaultDirName={autopf}\RustShot  ; Куда ставить
; {autopf} = Program Files для admin, AppData для обычного пользователя
PrivilegesRequired=lowest  ; Не требует admin прав
ArchitecturesInstallIn64BitMode=x64compatible  ; 64-бит
WizardStyle=modern         ; Современный вид (Windows 10+)
Compression=lzma           ; Хорошее сжатие
```

#### [Languages] — локализация

```iss
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
```

Inno Setup поставляется с переводами на 20+ языков.

#### [Files] — что копировать

```iss
[Files]
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
```

- `{app}` — путь установки (DefaultDirName)
- `ignoreversion` — всегда перезаписывать

Можно добавлять несколько файлов:

```iss
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
```

#### [Icons] — ярлыки

```iss
[Icons]
Name: "{group}\RustShot"; Filename: "{app}\rustshot.exe"
Name: "{group}\Uninstall RustShot"; Filename: "{uninstallexe}"
```

- `{group}` — папка в Start Menu
- Можно добавить ярлык на рабочий стол:

```iss
Name: "{userdesktop}\RustShot"; Filename: "{app}\rustshot.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create desktop shortcut"; GroupDescription: "Additional"
```

#### [Registry] — автозапуск

```iss
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
  ValueType: string; ValueName: "RustShot"; \
  ValueData: """{app}\rustshot.exe"""; Flags: uninsdeletevalue
```

- `HKCU` — текущий пользователь (не нужен admin)
- `uninsdeletevalue` — удалить при деинсталляции

#### [Run] — запуск после установки

```iss
[Run]
Filename: "{app}\rustshot.exe"; \
  Description: "Launch RustShot"; \
  Flags: nowait postinstall skipifsilent
```

### Сборка инсталлятора

```bash
# Локально
iscc installer.iss

# Результат
Output/RustShotSetup.exe
```

---

## 3. MSI через WiX

MSI — формат Microsoft. Используется в корпоративных средах (GPO, SCCM).

```xml
<!-- product.wxs -->
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="RustShot" Version="0.1.0"
           Manufacturer="RustShot Contributors"
           UpgradeCode="PUT-GUID-HERE">
    <Package InstallerVersion="200" Compressed="yes" />
    <Media Id="1" Cabinet="app.cab" EmbedCab="yes" />

    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLDIR" Name="RustShot">
          <Component Id="MainExecutable" Guid="PUT-GUID-HERE">
            <File Id="rustshot" Source="target\release\rustshot.exe" />
          </Component>
        </Directory>
      </Directory>
    </Directory>

    <Feature Id="Complete" Level="1">
      <ComponentRef Id="MainExecutable" />
    </Feature>
  </Product>
</Wix>
```

Сборка:

```bash
candle product.wxs
light product.wixobj -o RustShot.msi
```

---

## 4. cargo-wix

Обёртка над WiX специально для Rust:

```bash
cargo install cargo-wix
cargo wix init      # Генерирует wix/main.wxs
cargo wix            # Собирает MSI
```

Преимущество: автоматически подставляет имя, версию, лицензию из `Cargo.toml`.

---

## 5. NSIS

Альтернатива Inno Setup:

```nsis
!include "MUI2.nsh"

Name "RustShot"
OutFile "RustShotSetup.exe"
InstallDir "$PROGRAMFILES\RustShot"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath $INSTDIR
  File "target\release\rustshot.exe"
  CreateShortcut "$SMPROGRAMS\RustShot.lnk" "$INSTDIR\rustshot.exe"
SectionEnd
```

NSIS мощнее, но синтаксис менее читаемый. Для простых случаев Inno Setup удобнее.

---

## 6. Портативный .exe

Самый простой вариант — один файл, без установки:

```bash
cargo build --release
# target/release/rustshot.exe — готов к распространению
```

### Статическая линковка (для переносимости)

```bash
# Статически линкуем CRT (не нужен vcruntime140.dll)
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release
```

В `Cargo.toml`:

```toml
[profile.release]
opt-level = 3          # Максимальная оптимизация
lto = true             # Link-Time Optimization (меньше бинарник)
codegen-units = 1      # Лучшая оптимизация (медленнее компиляция)
strip = true           # Убрать debug символы
```

---

## 7. Автозапуск и трей

### Из Rust кода

```rust
use winreg::enums::*;
use winreg::RegKey;

fn set_autorun(enable: bool) -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE,
    )?;

    if enable {
        let exe_path = std::env::current_exe()?;
        run_key.set_value("RustShot", &exe_path.to_string_lossy().to_string())?;
    } else {
        let _ = run_key.delete_value("RustShot");
    }
    Ok(())
}
```

### Из инсталлятора (Inno Setup)

Преимущество: автозапуск автоматически удаляется при деинсталляции (флаг `uninsdeletevalue`).

---

## 8. Сборка в GitHub Actions

### Полный пример

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release

      - name: Build installer
        run: iscc installer.iss

      - uses: actions/upload-artifact@v4
        with:
          name: installer
          path: Output/RustShotSetup.exe

      - uses: softprops/action-gh-release@v2
        if: startsWith(github.ref, 'refs/tags/')
        with:
          files: Output/RustShotSetup.exe
```

### Автоматическая версия из тега

```iss
; В installer.iss
#define MyAppVersion GetEnv('APP_VERSION')
AppVersion={#MyAppVersion}
```

```yaml
# В workflow
- name: Set version from tag
  run: echo "APP_VERSION=${{ github.ref_name }}" >> $env:GITHUB_ENV

- run: iscc installer.iss
```

---

## 9. Linux: deb и AppImage

### deb-пакет через cargo-deb

```bash
cargo install cargo-deb
```

В `Cargo.toml`:

```toml
[package.metadata.deb]
maintainer = "Your Name <email>"
copyright = "2024, RustShot Contributors"
depends = "libxcb1, libgtk-3-0"
section = "utility"
assets = [
    ["target/release/rustshot", "usr/bin/", "755"],
]
```

```bash
cargo deb  # Создаёт target/debian/rustshot_0.1.0_amd64.deb
```

### AppImage (переносимый бинарник для Linux)

```bash
cargo install cargo-appimage
cargo appimage
```

---

## 10. Подпись кода

### Зачем

Без подписи Windows покажет SmartScreen: "Windows protected your PC".

### Самоподпись (для тестов)

```powershell
$cert = New-SelfSignedCertificate -Type CodeSigning -Subject "CN=RustShot"
Export-PfxCertificate -Cert $cert -FilePath code.pfx -Password (ConvertTo-SecureString -String "pass" -Force -AsPlainText)
signtool sign /f code.pfx /p pass /t http://timestamp.digicert.com target\release\rustshot.exe
```

### Коммерческий сертификат

Провайдеры: DigiCert, Sectigo, GlobalSign. Стоимость: $200-500/год.

В GitHub Actions:

```yaml
- name: Sign binary
  run: |
    signtool sign /f cert.pfx /p ${{ secrets.CERT_PASSWORD }} /t http://timestamp.digicert.com target\release\rustshot.exe
```

---

## Чеклист релиза

- [ ] `cargo build --release` компилируется
- [ ] `installer.iss` с правильной версией
- [ ] Тег `v0.1.0` создан и запушен
- [ ] GitHub Actions собрал артефакты
- [ ] GitHub Release создан с файлами
- [ ] Инсталлятор тестирован на чистой Windows
- [ ] Автозапуск работает после перезагрузки
- [ ] Деинсталляция чисто удаляет программу
