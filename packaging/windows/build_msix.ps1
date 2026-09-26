# Packs the Windows release folder into an MSIX with the Windows SDK's makeappx, and signs it when a
# certificate is given (docs/packaging.md). An unsigned MSIX cannot be installed.
#   pwsh packaging/windows/build_msix.ps1 -Release app\build\windows\x64\runner\Release `
#        -Version 0.1.0 -Out dist [-Publisher "CN=…"] [-Pfx cert.pfx -PfxPassword …]
param(
  [Parameter(Mandatory)] [string] $Release,
  [Parameter(Mandatory)] [string] $Version,
  [Parameter(Mandatory)] [string] $Out,
  [string] $Publisher = "CN=Daftar Test",
  [string] $Pfx = "",
  [string] $PfxPassword = ""
)
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
  Sort-Object FullName -Descending | Select-Object -First 1
if (-not $sdk) { throw "makeappx.exe not found; install the Windows 10/11 SDK." }
$bin = $sdk.DirectoryName

$stage = Join-Path ([IO.Path]::GetTempPath()) ("daftar-msix-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $stage | Out-Null
Copy-Item -Recurse -Path (Join-Path $Release "*") -Destination $stage
Copy-Item -Recurse -Path (Join-Path $here "Assets") -Destination $stage
# MSIX versions have four parts.
$v4 = (($Version -split '[.+-]')[0..2] -join '.') + '.0'
(Get-Content (Join-Path $here "AppxManifest.xml") -Raw).
  Replace('VERSION', $v4).Replace('PUBLISHER', $Publisher) |
  Set-Content -Encoding UTF8 (Join-Path $stage "AppxManifest.xml")

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$msix = Join-Path $Out "Daftar-$Version-x64.msix"
& (Join-Path $bin "makeappx.exe") pack /o /d $stage /p $msix
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
if ($Pfx) {
  & (Join-Path $bin "signtool.exe") sign /fd SHA256 /f $Pfx /p $PfxPassword $msix
  if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
}
Remove-Item -Recurse -Force $stage
Write-Output $msix
