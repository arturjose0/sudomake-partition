@echo off
rem Abre um Prompt de Comando como Administrador ja na pasta do macread e lista os discos.
rem (ler um disco fisico exige privilegios de Administrador)
if "%~1"=="elevado" goto :elevado
powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -ArgumentList 'elevado' -Verb RunAs"
goto :eof

:elevado
cd /d "%~dp0"
echo Pasta: %CD%
echo.
macread.exe discos
echo.
echo Exemplos:  macread info disco:1   ^|   macread ls disco:1 /Users   ^|   macread copiar disco:1 /Users D:\Recuperado
echo.
cmd /k
