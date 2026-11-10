#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

$installer = Join-Path $env:RUNNER_TEMP 'rustup-init.exe'
Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $installer
& $installer -y --profile minimal --default-host x86_64-pc-windows-msvc --default-toolchain stable
if ($LASTEXITCODE -ne 0) { throw "rustup-init failed: $LASTEXITCODE" }

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
"$cargoBin" | Out-File -FilePath $env:GITHUB_PATH -Append -Encoding utf8
$env:PATH = "$cargoBin;$env:PATH"
rustup component add rustfmt clippy

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }
cargo clippy --locked --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
cargo build --locked --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$exe = [System.IO.File]::ReadAllBytes((Resolve-Path 'target/release/morse-runner.exe'))
$peOffset = [BitConverter]::ToInt32($exe, 0x3C)
$subsystem = [BitConverter]::ToUInt16($exe, $peOffset + 24 + 68)
if ($subsystem -ne 2) { throw "Expected Windows GUI subsystem (2), got $subsystem" }

cargo install --locked --features cli cargo-about
if ($LASTEXITCODE -ne 0) { throw 'cargo-about install failed' }
New-Item -ItemType Directory -Force -Path target, dist | Out-Null
cargo about generate --fail about.hbs --output-file target/THIRD_PARTY-LICENSES.md
if ($LASTEXITCODE -ne 0) { throw 'cargo about failed' }

$version = $env:MORSE_RUNNER_BUILD_VERSION
if ([string]::IsNullOrWhiteSpace($version) -or
    $version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
  throw 'MORSE_RUNNER_BUILD_VERSION must be MAJOR.MINOR.PATCH'
}

$stage = Join-Path $env:RUNNER_TEMP 'morse-runner-installer'
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item target/release/morse-runner.exe $stage/
Copy-Item -Path @('README.md', 'LICENSE', 'THIRD_PARTY.md', 'target/THIRD_PARTY-LICENSES.md', 'dist/metadata/CHANGELOG.md', 'dist/metadata/BUILD-INFO.json') -Destination $stage

$portableName = "morse-runner_${version}_windows_x86_64.zip"
$portableZip = Join-Path (Resolve-Path dist) $portableName
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $portableZip -CompressionLevel Optimal
$archive = [System.IO.Compression.ZipFile]::OpenRead($portableZip)
try {
  $entries = @($archive.Entries | ForEach-Object { $_.FullName })
  foreach ($required in @('morse-runner.exe', 'README.md', 'LICENSE', 'THIRD_PARTY.md', 'THIRD_PARTY-LICENSES.md', 'CHANGELOG.md')) {
    if ($entries -notcontains $required) { throw "Portable ZIP is missing $required" }
  }
}
finally { $archive.Dispose() }

$setupName = "morse-runner_${version}_windows_x86_64_setup"
$iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
if (-not (Test-Path $iscc)) { throw 'Inno Setup compiler was not found on the Windows runner' }
& $iscc "/DAppVersion=$version" "/DStageDir=$stage" "/DOutputBaseFilename=$setupName" packaging/windows.iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed: $LASTEXITCODE" }

$setup = Join-Path (Resolve-Path dist) "$setupName.exe"
$installDir = Join-Path $env:RUNNER_TEMP 'morse-runner-install-smoke'
$installed = Start-Process -FilePath $setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/DIR=$installDir") -Wait -PassThru
if ($installed.ExitCode -ne 0 -or -not (Test-Path (Join-Path $installDir 'morse-runner.exe'))) { throw 'Inno Setup install smoke test failed' }
$uninstalled = Start-Process -FilePath (Join-Path $installDir 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -Wait -PassThru
if ($uninstalled.ExitCode -ne 0 -or (Test-Path $installDir)) { throw 'Inno Setup uninstall smoke test failed' }


