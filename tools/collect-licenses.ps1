param([Parameter(Mandatory=$true)][string]$Metadata)
$ErrorActionPreference = 'Stop'
$project = Split-Path $PSScriptRoot -Parent
$graph = Get-Content -LiteralPath $Metadata -Raw -Encoding UTF8 | ConvertFrom-Json
$packages = @{}; $nodes = @{}
foreach($package in $graph.packages) { $packages[$package.id] = $package }
foreach($node in $graph.resolve.nodes) { $nodes[$node.id] = $node }
$seen = New-Object 'System.Collections.Generic.HashSet[string]'
$queue = New-Object 'System.Collections.Generic.Queue[string]'
$queue.Enqueue($graph.resolve.root)
while($queue.Count -gt 0) {
    $id = $queue.Dequeue()
    if (!$seen.Add($id)) { continue }
    foreach($dependency in $nodes[$id].deps) {
        if (@($dependency.dep_kinds | Where-Object { $null -eq $_.kind }).Count -gt 0) { $queue.Enqueue($dependency.pkg) }
    }
}
$text = New-Object System.Text.StringBuilder
[void]$text.AppendLine("Rust runtime dependencies / Windows x64`nGenerated from Cargo.lock; build-only dependencies excluded.`nSlint components use the Slint Royalty-free License 2.0, reproduced separately.`n")
foreach($id in @($seen) | Sort-Object) {
    if ($id -eq $graph.resolve.root) { continue }
    $package = $packages[$id]
    [void]$text.AppendLine("`n============================================================`n$($package.name) $($package.version)`nLicense expression: $($package.license)`n$($package.repository)`n")
    $folder = Split-Path $package.manifest_path -Parent
    $files = @(Get-ChildItem -LiteralPath $folder -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)([.\-_]|$)' })
    foreach($subfolder in @('licenses','LICENSES')) {
        $path = Join-Path $folder $subfolder
        if (Test-Path -LiteralPath $path -PathType Container) { $files += @(Get-ChildItem -LiteralPath $path -File) }
    }
    foreach($file in $files | Sort-Object FullName -Unique) {
        [void]$text.AppendLine("--- $($file.Name) ---")
        [void]$text.AppendLine([IO.File]::ReadAllText($file.FullName))
    }
    if ($files.Count -eq 0) { [void]$text.AppendLine('See the package source and repository for license terms.') }
}
$bytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($text.ToString())
$target = Join-Path $project 'licenses\Rust-dependencies.txt.gz'
$stream = [IO.File]::Create($target)
$gzip = New-Object IO.Compression.GZipStream($stream,[IO.Compression.CompressionLevel]::Optimal)
$gzip.Write($bytes,0,$bytes.Length); $gzip.Dispose(); $stream.Dispose()
Write-Output "Collected $($seen.Count - 1) runtime package notices, $($bytes.Length) uncompressed bytes."
