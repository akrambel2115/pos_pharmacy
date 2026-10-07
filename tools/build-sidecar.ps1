<#
.SYNOPSIS
  Builds the standalone OCR Invoice Extractor sidecar executable for Tauri.
.DESCRIPTION
  Compiles src-tauri/engine/invoice_extractor.py into a standalone .exe with PyInstaller,
  bundling Python, ONNX runtime, and RapidOCR models so end users do not need Python.
#>

$ErrorActionPreference = 'Stop'
$rootDir = (Get-Item $PSScriptRoot).Parent.FullName
Set-Location $rootDir

Write-Host "Checking / installing Python dependencies for Sidecar..." -ForegroundColor Cyan
python -m pip install --upgrade pyinstaller rapidocr_onnxruntime onnxruntime pypdfium2 numpy

$binDir = Join-Path $rootDir "src-tauri\bin"
if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir | Out-Null
}

$scriptPath = Join-Path $rootDir "src-tauri\engine\invoice_extractor.py"
$tempWorkDir = Join-Path $rootDir "src-tauri\build_pyinstaller"

Write-Host "Compiling standalone OCR binary with PyInstaller..." -ForegroundColor Cyan
python -m PyInstaller --noconfirm --clean --onefile `
    --collect-all rapidocr_onnxruntime `
    --collect-all onnxruntime `
    --collect-all pypdfium2 `
    --collect-all numpy `
    --distpath $binDir `
    --workpath $tempWorkDir `
    --name "invoice_extractor-x86_64-pc-windows-msvc" `
    $scriptPath

# Cleanup temporary work directory and spec file
if (Test-Path $tempWorkDir) {
    Remove-Item -Recurse -Force $tempWorkDir
}
$specFile = Join-Path $rootDir "invoice_extractor-x86_64-pc-windows-msvc.spec"
if (Test-Path $specFile) {
    Remove-Item -Force $specFile
}

Write-Host "Sidecar successfully built at: $binDir\invoice_extractor-x86_64-pc-windows-msvc.exe" -ForegroundColor Green
