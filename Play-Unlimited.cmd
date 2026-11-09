@echo off
setlocal
cd /d "%~dp0"
rem Prefer the installed Rustup toolchain over an older Rust/GCC installation.
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "DRDRIVING_UNLIMITED_COINS=1"
cargo run --release -p drdriving
if errorlevel 1 pause
endlocal
