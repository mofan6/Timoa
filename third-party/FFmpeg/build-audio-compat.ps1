param(
    [Parameter(Mandatory=$true)][string]$ToolchainBin,
    [Parameter(Mandatory=$true)][string]$SourceDirectory,
    [Parameter(Mandatory=$true)][string]$BuildDirectory,
    [ValidateSet('x86','x64')][string]$Architecture='x86',
    [int]$Jobs=4
)
$ErrorActionPreference='Stop'
$env:PATH=$ToolchainBin+';'+$env:PATH
New-Item -ItemType Directory -Path $BuildDirectory -Force | Out-Null
$taskPrefix=if ($Architecture -eq 'x86') {'i686-w64-mingw32-'} else {'x86_64-w64-mingw32-'}
$taskArch=if ($Architecture -eq 'x86') {'x86_32'} else {'x86_64'}
$taskFlags=@(Get-Content -LiteralPath (Join-Path $PSScriptRoot 'configure-flags.txt') | Where-Object { $_.Trim() })
$taskFlags=$taskFlags | ForEach-Object {
    if ($_ -like '--arch=*') { "--arch=$taskArch" }
    elseif ($_ -like '--cc=*') { "--cc=$($taskPrefix)gcc" }
    elseif ($_ -like '--ld=*') { "--ld=$($taskPrefix)gcc" }
    elseif ($_ -like '--extra-cflags=*') { $_+' -D_WIN32_WINNT=0x0601 -DWINVER=0x0601' }
    elseif ($_ -like '--extra-version=*') { "--extra-version=Timoa-audio-win7-$Architecture" }
    else { $_ }
}
foreach ($taskFlag in $taskFlags) { if ($taskFlag.Contains("'")) { throw 'Unexpected shell quote in configure flags.' } }
$taskSource=$SourceDirectory.Replace('\','/')
$taskBuild=$BuildDirectory.Replace('\','/')
if ($taskSource.Contains("'") -or $taskBuild.Contains("'")) { throw 'Use build paths without single quotes.' }
$taskCommand="#!/bin/sh`nset -eu`nexport TMPDIR='$taskBuild/tmp'`nmkdir -p `"`$TMPDIR`"`ncd '$taskBuild'`n'$taskSource/configure'"
foreach ($taskFlag in $taskFlags) { $taskCommand+=' '+"'$taskFlag'" }
$taskCommand+="`nmake V=1 -j$Jobs ffmpeg.exe`nstrip --strip-all ffmpeg.exe`n"
$taskScript=Join-Path $BuildDirectory 'build-decoder.sh'
[IO.File]::WriteAllText($taskScript,$taskCommand,[Text.UTF8Encoding]::new($false))
& (Join-Path $ToolchainBin 'sh.exe') $taskScript
if ($LASTEXITCODE -ne 0) { throw 'Audio decoder build failed.' }
