@echo off
rem Compila o SUDOMAKE Partition (linha de comando + interface grafica) em modo release usando a
rem toolchain GNU do Rust. Nao precisa do Visual Studio.
where rustup >nul 2>&1 || (echo Instale o Rust primeiro: https://rustup.rs & exit /b 1)
rustup toolchain list | findstr /C:"stable-x86_64-pc-windows-gnu" >nul || rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
cargo +stable-x86_64-pc-windows-gnu build --release
if errorlevel 1 exit /b 1
copy /Y target\release\sudomake-partition.exe sudomake-partition.exe >nul
copy /Y target\release\SudomakePartition.exe SudomakePartition.exe >nul
echo.
echo Pronto: %CD%\SudomakePartition.exe (interface)  e  %CD%\sudomake-partition.exe (linha de comando)
