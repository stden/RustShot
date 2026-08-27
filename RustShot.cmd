@echo off
echo Building RustShot in release mode...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    exit /b %ERRORLEVEL%
)

copy /y target\release\RustShot.exe . > nul
echo Done! Final executable is at C:\x\RustShot\RustShot.exe
echo Running RustShot...
.\RustShot.exe
