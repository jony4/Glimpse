$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build --release --locked --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed' }
$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'glim-app').version
$stage = Join-Path $PWD 'dist/windows-x64/Glim'
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item (Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc/release/glim.exe') (Join-Path $stage 'Glim.exe') -Force
$smoke = Start-Process -FilePath (Join-Path $stage 'Glim.exe') -ArgumentList '--help' -Wait -PassThru -RedirectStandardOutput (Join-Path $PWD 'dist/windows-help.txt') -RedirectStandardError (Join-Path $PWD 'dist/windows-help-error.txt')
if ($smoke.ExitCode -ne 0) { throw "Packaged executable startup failed: $($smoke.ExitCode)" }
Copy-Item LICENSE (Join-Path $stage 'LICENSE.txt') -Force
@"
Glim $version - Windows x64
Extract this folder and open Glim.exe. Git features require Git for Windows on PATH.
Shortcuts: Ctrl+O open file, Ctrl+Shift+O open folder, Ctrl+S save, Ctrl+Shift+N new window.
Windows supports the main file/text/image/Git workspace. macOS-native document, media,
font and SQLite preview windows and the default-file-type manager are not included.
This executable is not Authenticode signed. Download only from the project release page.
https://github.com/jony4/Glim/releases
"@ | Set-Content (Join-Path $stage 'Read Me.txt') -Encoding UTF8
$archive = Join-Path $PWD "dist/Glim-$version-windows-x64.zip"
Compress-Archive -Path $stage -DestinationPath $archive -Force
$hash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $([System.IO.Path]::GetFileName($archive))" | Set-Content "$archive.sha256" -Encoding ascii
Write-Output $archive
