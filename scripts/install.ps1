[CmdletBinding()]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\clash-go'),
    [ValidateSet('auto', 'direct', 'mirror')]
    [string]$Route = 'auto',
    [string]$MirrorBaseUrl,
    [switch]$NoPath,
    [switch]$Launch
)

$ErrorActionPreference = 'Stop'
$repo = 'neko233-com/clash-go'
$api = "https://api.github.com/repos/$repo"
$defaultMirror = 'https://gh-proxy.org/'
$userAgent = 'clash-go-install/1.0'

function Get-RouteCandidates {
    $candidates = @(@{ Name = 'direct'; Prefix = '' })
    if ($MirrorBaseUrl) {
        $mirrors = @($MirrorBaseUrl)
    } elseif ($env:CLASH_GO_GITHUB_MIRRORS) {
        $mirrors = @($env:CLASH_GO_GITHUB_MIRRORS -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    } else {
        $mirrors = @($defaultMirror)
    }
    $index = 0
    foreach ($mirror in $mirrors) {
        $index++
        $candidates += @{ Name = "mirror-$index"; Prefix = ($mirror.TrimEnd('/') + '/') }
    }
    return $candidates
}

function Get-ProbeResults {
    $results = @()
    foreach ($candidate in (Get-RouteCandidates)) {
        if ($Route -eq 'direct' -and $candidate.Name -ne 'direct') { continue }
        if ($Route -eq 'mirror' -and $candidate.Name -eq 'direct') { continue }
        $url = $candidate.Prefix + $api
        $timer = [Diagnostics.Stopwatch]::StartNew()
        try {
            $null = Invoke-RestMethod -Uri $url -Headers @{ 'User-Agent' = $userAgent; Accept = 'application/vnd.github+json' } -TimeoutSec 6
            $timer.Stop()
            $results += [pscustomobject]@{ Name = $candidate.Name; Prefix = $candidate.Prefix; LatencyMs = $timer.ElapsedMilliseconds; Reachable = $true; Detail = $null }
        } catch {
            $timer.Stop()
            $results += [pscustomobject]@{ Name = $candidate.Name; Prefix = $candidate.Prefix; LatencyMs = $timer.ElapsedMilliseconds; Reachable = $false; Detail = $_.Exception.Message }
        }
    }
    return $results
}

Write-Host 'Checking direct GitHub and mirror routes...'
$routes = @(Get-ProbeResults)
$routes | ForEach-Object { Write-Host ('  {0,-12} {1,5} ms  {2}' -f $_.Name, $_.LatencyMs, $(if ($_.Reachable) { 'reachable' } else { $_.Detail })) }
$selected = $routes | Where-Object { $_.Reachable } | Sort-Object LatencyMs | Select-Object -First 1
if (-not $selected) { throw 'GitHub is unreachable through the selected route(s). Set CLASH_GO_GITHUB_MIRRORS or -MirrorBaseUrl and retry.' }
Write-Host "Using $($selected.Name)."

# Prefer official GitHub for release metadata and its SHA-256 digest whenever it is reachable.
$metadataRoute = $routes | Where-Object { $_.Name -eq 'direct' -and $_.Reachable } | Select-Object -First 1
if (-not $metadataRoute) { $metadataRoute = $selected }
$releaseUrl = $metadataRoute.Prefix + "$api/releases/latest"
try {
    $release = Invoke-RestMethod -Uri $releaseUrl -Headers @{ 'User-Agent' = $userAgent; Accept = 'application/vnd.github+json' } -TimeoutSec 12
} catch {
    if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 404) {
        throw 'There is no published GitHub release yet. Install after the first v* release is published.'
    }
    throw "Could not read the latest GitHub release: $($_.Exception.Message)"
}

$asset = $release.assets | Where-Object { $_.name -eq 'clash-go-windows-x64.zip' } | Select-Object -First 1
if (-not $asset) { throw "Release $($release.tag_name) has no clash-go-windows-x64.zip asset." }
$expectedDigest = $asset.digest
if (-not $expectedDigest -or $expectedDigest -notmatch '^sha256:[0-9a-fA-F]{64}$') {
    $sumAsset = $release.assets | Where-Object { $_.name -eq 'SHA256SUMS' } | Select-Object -First 1
    if (-not $sumAsset) { throw 'Release does not publish an SHA-256 digest or SHA256SUMS; refusing an unverifiable install.' }
    $sumText = (Invoke-WebRequest -Uri ($metadataRoute.Prefix + $sumAsset.browser_download_url) -Headers @{ 'User-Agent' = $userAgent } -UseBasicParsing -TimeoutSec 20).Content
    $sumLine = $sumText -split "`r?`n" | Where-Object { $_ -match 'clash-go-windows-x64\.zip\s*$' } | Select-Object -First 1
    if (-not $sumLine -or $sumLine -notmatch '^([0-9a-fA-F]{64})\s+\*?clash-go-windows-x64\.zip\s*$') { throw 'Could not find the installer ZIP hash in SHA256SUMS.' }
    $expectedDigest = 'sha256:' + $Matches[1]
}
$expectedHash = ($expectedDigest -replace '^sha256:', '').ToLowerInvariant()

$workDir = Join-Path ([IO.Path]::GetTempPath()) ('clash-go-install-' + [guid]::NewGuid().ToString('N'))
$stageDir = Join-Path $workDir 'stage'
$zipPath = Join-Path $workDir $asset.name
New-Item -ItemType Directory -Path $stageDir -Force | Out-Null
try {
    $assetUrl = $selected.Prefix + $asset.browser_download_url
    Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -Headers @{ 'User-Agent' = $userAgent } -UseBasicParsing -TimeoutSec 180
    $actualHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) { throw "SHA-256 verification failed. Expected $expectedHash, got $actualHash." }
    Expand-Archive -LiteralPath $zipPath -DestinationPath $stageDir -Force
    $newExe = Join-Path $stageDir 'clash-go.exe'
    if (-not (Test-Path -LiteralPath $newExe -PathType Leaf)) { throw 'The verified archive does not contain clash-go.exe.' }

    $targetExe = Join-Path $InstallDir 'clash-go.exe'
    $resolvedTarget = [IO.Path]::GetFullPath($targetExe)
    $running = Get-Process -Name 'clash-go' -ErrorAction SilentlyContinue | Where-Object {
        try { [IO.Path]::GetFullPath($_.Path) -ieq $resolvedTarget } catch { $false }
    }
    foreach ($process in $running) { Stop-Process -Id $process.Id -Force }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -Path (Join-Path $stageDir '*') -Destination $InstallDir -Recurse -Force
    if (-not $NoPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pathEntries = @($userPath -split ';' | Where-Object { $_ })
        if ($pathEntries -notcontains $InstallDir) {
            [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $InstallDir) -join ';'), 'User')
            Write-Host 'Added the install directory to the user PATH (new terminals will pick it up).'
        }
    }
    Write-Host "Installed clash-go $($release.tag_name) to $InstallDir"
    if ($Launch) { Start-Process -FilePath $targetExe }
} finally {
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
