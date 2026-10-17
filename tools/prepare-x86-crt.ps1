param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference='Stop'
if ((Test-Path -LiteralPath (Join-Path $Destination 'libmingw32.a')) -and
    (Test-Path -LiteralPath (Join-Path $Destination 'libgcc_eh.a'))) { return }
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$taskArchive=Join-Path $Destination 'rust-mingw-1.99.0-i686-pc-windows-gnu.tar.xz'
$taskHash='A6855Eebcb2b8550a27d7cde8ea03f7447326f9fdd0b51905241663cf3f06fcf'
$taskUrl='https://static.rust-lang.org/dist/2026-10-01/rust-mingw-1.99.0-i686-pc-windows-gnu.tar.xz'
if (!(Test-Path -LiteralPath $taskArchive)) {
    # Native build input only; no precompiled Rust std or application runtime.
    Invoke-WebRequest -Uri $taskUrl -OutFile $taskArchive -UseBasicParsing
}
if ((Get-FileHash -LiteralPath $taskArchive -Algorithm SHA256).Hash -ne $taskHash) {
    throw 'The official 32-bit MinGW component does not match the pinned SHA256.'
}
$taskMember='rust-mingw-1.99.0-i686-pc-windows-gnu/rust-mingw/lib/rustlib/i686-pc-windows-gnu/lib/self-contained'
& tar.exe -xf $taskArchive -C $Destination --strip-components=7 $taskMember
if ($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath (Join-Path $Destination 'libmingw32.a'))) {
    throw 'Unable to extract the matching 32-bit native CRT.'
}
