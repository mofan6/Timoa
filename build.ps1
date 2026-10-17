param(
    [ValidateSet('x64','x86','All')][string]$Architecture='All',
    [switch]$Setup,
    [string]$ToolchainBin,
    [string]$NsisCompiler,
    [string]$Upx
)
$ErrorActionPreference='Stop'
Set-Location -LiteralPath $PSScriptRoot
$taskPackageVersion = [regex]::Match([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'Cargo.toml')), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (!$taskPackageVersion) { throw 'Cannot read the application version from Cargo.toml.' }
if ($ToolchainBin) { $env:PATH=$ToolchainBin+';'+$env:PATH }
if (!(Get-Command rustc -ErrorAction SilentlyContinue)) { throw 'Install Rust 1.99.0 x86_64-pc-windows-gnu and rust-src.' }
$taskVersion=& rustc -vV
if ($LASTEXITCODE -ne 0) { throw 'Cannot read Rust toolchain information.' }
if (!($taskVersion -match '^host: x86_64-pc-windows-gnu$') -or !($taskVersion -match '^release: 1.99.0$')) {
    throw 'Use Rust 1.99.0-x86_64-pc-windows-gnu with rust-src (see WINDOWS-SUPPORT.md).'
}
foreach ($taskCommand in @('i686-w64-mingw32-gcc','x86_64-w64-mingw32-gcc','i686-w64-mingw32-windres','x86_64-w64-mingw32-windres')) {
    if (!(Get-Command $taskCommand -ErrorAction SilentlyContinue)) { throw "Add w64devkit/bin to PATH: missing $taskCommand" }
}
$taskSysroot=(& rustc --print sysroot).Trim()
$taskLinker=(Join-Path $taskSysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin\rust-lld.exe').Replace('\','/')
$taskTargetDir=if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $PSScriptRoot 'target-compat' }
$env:CARGO_TARGET_DIR=$taskTargetDir
$env:RUSTC_BOOTSTRAP='1'
New-Item -ItemType Directory -Path $taskTargetDir -Force | Out-Null
$taskHostConfig=Join-Path $taskTargetDir 'cargo-host.toml'
$taskHostText=@"
target-applies-to-host = false
[host]
linker = "$taskLinker"
rustflags = ["-C", "linker-flavor=ld.lld", "-C", "link-self-contained=yes", "-C", "target-feature=+crt-static"]
"@
[IO.File]::WriteAllText($taskHostConfig,$taskHostText)
$taskArchitectures=if ($Architecture -eq 'All') { @('x64','x86') } else { @($Architecture) }
foreach ($taskArch in $taskArchitectures) {
    $taskTarget=if ($taskArch -eq 'x86') { 'i686-win7-windows-gnu' } else { 'x86_64-win7-windows-gnu' }
    $taskCrtBase=if ($taskArch -eq 'x86') { 'i686-pc-windows-gnu' } else { 'x86_64-pc-windows-gnu' }
    if ($taskArch -eq 'x86') {
        # The cross-target std package contains only startup objects, not the
        # complete 32-bit MinGW CRT. Acquire only the matching native component.
        $taskCrtSource=Join-Path $taskTargetDir 'native-crt-x86'
        & (Join-Path $PSScriptRoot 'tools\prepare-x86-crt.ps1') -Destination $taskCrtSource
        $taskStartupSource=Join-Path $taskSysroot 'lib\rustlib\i686-pc-windows-gnu\lib\self-contained'
        foreach ($taskObject in @('crt2.o','dllcrt2.o')) {
            $taskObjectPath=Join-Path $taskStartupSource $taskObject
            if (!(Test-Path -LiteralPath $taskObjectPath)) { throw 'Run rustup target add --toolchain 1.99.0-x86_64-pc-windows-gnu i686-pc-windows-gnu for CRT startup objects.' }
            Copy-Item -LiteralPath $taskObjectPath -Destination (Join-Path $taskCrtSource $taskObject) -Force
        }
    } else {
        $taskCrtSource=Join-Path $taskSysroot "lib\rustlib\$taskCrtBase\lib\self-contained"
    }
    $taskCrtTarget=Join-Path $taskSysroot "lib\rustlib\$taskTarget\lib\self-contained"
    if (!(Test-Path -LiteralPath (Join-Path $taskCrtSource 'libmingw32.a'))) { throw "Install the complete Rust GNU toolchain for $taskCrtBase, including rust-mingw." }
    # Copy CRT startup/link libraries only. Win7 std is built from rust-src below.
    New-Item -ItemType Directory -Path $taskCrtTarget -Force | Out-Null
    Get-ChildItem -LiteralPath $taskCrtSource -File | Where-Object { $_.Extension -in @('.a','.o') } | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $taskCrtTarget -Force }
    $taskFlags=@('-C',"linker=$taskLinker",'-C','linker-flavor=ld.lld','-C','link-self-contained=yes','-C','target-feature=+crt-static',
        '--cfg','windows_slim_errors','-C','link-arg=--major-subsystem-version=6','-C','link-arg=--minor-subsystem-version=1')
    $env:CARGO_ENCODED_RUSTFLAGS=$taskFlags -join [char]31
    $env:CC_i686_win7_windows_gnu='i686-w64-mingw32-gcc'
    $env:CC_x86_64_win7_windows_gnu='x86_64-w64-mingw32-gcc'
    $env:AR_i686_win7_windows_gnu='ar'
    $env:AR_x86_64_win7_windows_gnu='ar'
    & cargo build --release --locked --bin Timoa --target $taskTarget -Z build-std=std,panic_abort -Z host-config -Z target-applies-to-host --config $taskHostConfig
    if ($LASTEXITCODE -ne 0) { throw "$taskArch build failed." }
    $taskDist=Join-Path $PSScriptRoot "dist\windows-$taskArch"
    New-Item -ItemType Directory -Path $taskDist -Force | Out-Null
    $taskExe=Join-Path $taskDist "Timoa-$taskPackageVersion-windows-$taskArch.exe"
    Copy-Item -LiteralPath (Join-Path $taskTargetDir "$taskTarget\release\Timoa.exe") -Destination $taskExe -Force
    if ($Upx) {
        & $Upx --best --lzma --compress-icons=0 --strip-relocs=0 $taskExe
        if ($LASTEXITCODE -ne 0) { throw "$taskArch executable compression failed." }
    }
    if ($Setup) {
        if (!$NsisCompiler) {
            $taskNsis=Get-Command makensis.exe -ErrorAction SilentlyContinue
            if (!$taskNsis) { throw 'Pass -NsisCompiler for an NSIS compiler with x86/amd64 Unicode stubs and System plugins.' }
            $NsisCompiler=$taskNsis.Source
        }
        & $NsisCompiler /V3 /INPUTCHARSET UTF8 /OUTPUTCHARSET UTF8 "/DAPP_VERSION=$taskPackageVersion" "/DAPP_ARCH=$taskArch" "/DAPP_EXE=$taskExe" "/DPACKAGE_DIR=$taskDist" 'installer\Timoa.nsi'
        if ($LASTEXITCODE -ne 0) { throw "$taskArch installer build failed." }
    }
    Get-ChildItem -LiteralPath $taskDist -File | Select-Object Name,Length
}
