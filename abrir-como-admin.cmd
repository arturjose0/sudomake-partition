@echo off
rem Abre um Prompt de Comando como Administrador ja na pasta do SUDOMAKE Partition e lista os discos.
rem (ler um disco fisico exige privilegios de Administrador)
if "%~1"=="elevado" goto :elevado
powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -ArgumentList 'elevado' -Verb RunAs"
goto :eof

:elevado
cd /d "%~dp0"
chcp 65001 >nul
echo SUDOMAKE Partition - linha de comando (feito em Angola por Jose Artur Kassala / SUDOMAKE)
echo Pasta: %CD%
echo.
sudomake-partition.exe discos
echo.
echo Exemplos:  sudomake-partition info disco:1   ^|   sudomake-partition ls disco:1 /Users   ^|   sudomake-partition copiar disco:1 /Users D:\Recuperado
echo.
cmd /k
