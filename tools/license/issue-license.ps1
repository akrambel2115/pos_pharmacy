<#
.SYNOPSIS
  Vendor-side license generator for PharmaPOS (run on YOUR machine only).

.DESCRIPTION
  1. Asks for the Machine Code shown on the client's activation screen.
  2. Asks for the license period (number of days, or F for forever).
  3. Prints a signed License Code to send back to the client.

  On first run it creates an ECDSA P-256 key pair next to this script:
    license-private.key  -> SECRET. Never share, never commit, back it up.
    license-public.txt   -> Public key. Must be embedded in src-tauri/src/license.rs
                            (LICENSE_PUBLIC_KEY_B64). Already done if you did not delete the keys.

  If you lose license-private.key you can no longer issue licenses for already-shipped builds.
#>

$ErrorActionPreference = 'Stop'
$dir      = Split-Path -Parent $MyInvocation.MyCommand.Path
$privPath = Join-Path $dir 'license-private.key'
$pubPath  = Join-Path $dir 'license-public.txt'

function ConvertTo-B64Url([byte[]]$bytes) {
    return [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
}

# ---- Load or create key pair -------------------------------------------------
if (Test-Path $privPath) {
    $blob = [Convert]::FromBase64String((Get-Content $privPath -Raw).Trim())
    $key  = [System.Security.Cryptography.CngKey]::Import($blob, [System.Security.Cryptography.CngKeyBlobFormat]::EccPrivateBlob)
} else {
    $cp = New-Object System.Security.Cryptography.CngKeyCreationParameters
    $cp.ExportPolicy = [System.Security.Cryptography.CngExportPolicies]::AllowPlaintextExport
    $key = [System.Security.Cryptography.CngKey]::Create([System.Security.Cryptography.CngAlgorithm]::ECDsaP256, $null, $cp)
    [IO.File]::WriteAllText($privPath, [Convert]::ToBase64String($key.Export([System.Security.Cryptography.CngKeyBlobFormat]::EccPrivateBlob)))
    Write-Host "New key pair created: $privPath" -ForegroundColor Yellow
}

# Public key = X||Y (64 bytes) = EccPublicBlob without its 8-byte header
$pubBlob = $key.Export([System.Security.Cryptography.CngKeyBlobFormat]::EccPublicBlob)
$pubB64  = [Convert]::ToBase64String($pubBlob[8..($pubBlob.Length - 1)])
[IO.File]::WriteAllText($pubPath, $pubB64)

Write-Host ''
Write-Host '=== PharmaPOS License Generator ===' -ForegroundColor Cyan

# ---- Machine code ------------------------------------------------------------
$machine = (Read-Host 'Machine code from client').Trim().ToUpper()
if ($machine -notmatch '^[0-9A-F]{4}(-[0-9A-F]{4}){4}$') {
    Write-Host 'Invalid machine code format (expected XXXX-XXXX-XXXX-XXXX-XXXX).' -ForegroundColor Red
    exit 1
}

# ---- Period ------------------------------------------------------------------
$period = (Read-Host 'Period in days (e.g. 30, 365) or F for forever').Trim()
$issued = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
if ($period -match '^[fF]') {
    $expiry = 0
    $label  = 'FOREVER'
} elseif ($period -match '^\d+$' -and [int]$period -gt 0) {
    $expiry = [DateTimeOffset]::UtcNow.AddDays([int]$period).ToUnixTimeSeconds()
    $label  = "$period day(s) - expires " + [DateTimeOffset]::FromUnixTimeSeconds($expiry).LocalDateTime.ToString('yyyy-MM-dd HH:mm')
} else {
    Write-Host 'Invalid period.' -ForegroundColor Red
    exit 1
}

# ---- Sign --------------------------------------------------------------------
$payload = "PH1|$machine|$issued|$expiry"
$ecdsa   = New-Object System.Security.Cryptography.ECDsaCng($key)
$ecdsa.HashAlgorithm = [System.Security.Cryptography.CngAlgorithm]::Sha256
$sig     = $ecdsa.SignData([Text.Encoding]::UTF8.GetBytes($payload))   # 64 bytes, r||s

$token = (ConvertTo-B64Url ([Text.Encoding]::UTF8.GetBytes($payload))) + '.' + (ConvertTo-B64Url $sig)

Write-Host ''
Write-Host "Machine : $machine"
Write-Host "Period  : $label"
Write-Host ''
Write-Host 'License code (send this to the client):' -ForegroundColor Green
Write-Host ''
Write-Host $token -ForegroundColor White
Write-Host ''
try { Set-Clipboard -Value $token; Write-Host '(copied to clipboard)' -ForegroundColor DarkGray } catch {}
