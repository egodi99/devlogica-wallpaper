# Compila DevLogica Wallpaper per Windows e prepara la cartella "dist".
# Uso (PowerShell, nella cartella del progetto):  .\packaging\windows\build.ps1
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..\..")

cargo build --release
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item target\release\devlogica-wallpaper.exe "dist\DevLogica Wallpaper.exe" -Force
Write-Host "Creato: dist\DevLogica Wallpaper.exe"
Write-Host "Basta copiare questo file sui PC: non serve installazione."
