@echo off
rem Compila o macread (linha de comando + interface grafica) em modo release usando a toolchain GNU do Rust.
rem Nao precisa do Visual Studio.
where rustup >nul 2>&1 || (echo Instale o Rust primeiro: https://rustup.rs & exit /b 1)
rustup toolchain list | findstr /C:"stable-x86_64-pc-windows-gnu" >nul || rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
cargo +stable-x86_64-pc-windows-gnu build --release
if errorlevel 1 exit /b 1
copy /Y target\release\macread.exe macread.exe >nul
copy /Y target\release\macread-gui.exe macread-gui.exe >nul
echo.
echo Pronto: %CD%\macread.exe  e  %CD%\macread-gui.exe
