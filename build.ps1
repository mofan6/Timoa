param([switch]$Setup, [string]$NsisCompiler, [string]$Upx)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$target = (& rustc -vV | Select-String '^host: ').ToString().Substring(6).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Install the Rust toolchain first.' }
if ($target -eq 'x86_64-pc-windows-gnu') {
    $sysroot = (& rustc --print sysroot).Trim()
    $linker = (Join-Path $sysroot "lib\rustlib\$target\bin\rust-lld.exe").Replace('\','/')
    $flags = @('-C', "linker=$linker", '-C', 'linker-flavor=ld.lld', '-C', 'link-self-contained=yes', '-C', 'target-feature=+crt-static')
    if (!(Get-Command windres -ErrorAction SilentlyContinue)) { throw 'Add w64devkit/bin to PATH (resource compiler and Windows shell import library).' }
} elseif ($target -eq 'x86_64-pc-windows-msvc') {
    $flags = @('-C', 'target-feature=+crt-static')
} else { throw "Use an x64 Windows Rust toolchain. Current: $target" }
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
& cargo build --release --locked --bin Timoa
if ($LASTEXITCODE -ne 0) { throw 'Native build failed.' }
[IO.Directory]::CreateDirectory((Join-Path $PSScriptRoot 'dist')) | Out-Null
$targetDirectory = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
$builtExe = Join-Path $targetDirectory 'release\Timoa.exe'
Copy-Item -LiteralPath $builtExe -Destination 'dist\Timoa.exe'
if ($Upx) {
    & $Upx --best --lzma --compress-icons=0 --strip-relocs=0 'dist\Timoa.exe'
    if ($LASTEXITCODE -ne 0) { throw 'Executable compression failed.' }
}
if ($Setup) {
    if (!$NsisCompiler) {
        $command = Get-Command makensis.exe -ErrorAction SilentlyContinue
        if ($command) { $NsisCompiler = $command.Source }
        else { $NsisCompiler = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
    }
    if (!(Test-Path -LiteralPath $NsisCompiler)) { throw 'Pass -NsisCompiler with the path to NSIS 3.11+ makensis.exe.' }
    $exe = Join-Path $PSScriptRoot 'dist\Timoa.exe'
    $output = Join-Path $PSScriptRoot 'dist'
    & $NsisCompiler /V3 /INPUTCHARSET UTF8 /OUTPUTCHARSET UTF8 "/DAPP_EXE=$exe" "/DPACKAGE_DIR=$output" 'installer\Timoa.nsi'
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
}
Get-ChildItem -LiteralPath 'dist' -File | Select-Object Name,Length
