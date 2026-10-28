param([switch]$NoBuild, [string]$WorkspaceRoot)
$ErrorActionPreference = 'Stop'
$projectRoot = if ($WorkspaceRoot) { [IO.Path]::GetFullPath($WorkspaceRoot) } else { [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')) }
Push-Location $projectRoot
try {
    if (!$NoBuild) {
        & cargo build -p atelier-ui --bin Atelier
        if ($LASTEXITCODE -ne 0) { throw 'Atelier build failed' }
    }
    & (Join-Path $projectRoot 'target/debug/Atelier.exe') --dock-recovery-test
    if ($LASTEXITCODE -ne 0) { throw 'Dock recovery test failed' }
    $testRoot = Join-Path $projectRoot ('target/dock-native-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    $testExe = Join-Path $testRoot 'Atelier.exe'
    Copy-Item -LiteralPath (Join-Path $projectRoot 'target/debug/Atelier.exe') -Destination $testExe
    $testLink = Join-Path $testRoot 'Fixture.lnk'
    & $testExe --dock-write-link $testLink
    if ($LASTEXITCODE -ne 0) { throw 'Could not prepare launch fixture' }
    $settings = @{enabled=$false;hide_windows_taskbar=$false;imported=$true;pinned=@(@{path=$testLink;name='Atelier launch fixture'})} | ConvertTo-Json -Depth 4
    [IO.File]::WriteAllText((Join-Path $testRoot 'dock-settings.json'), $settings)
    $oldData = $env:ATELIER_DATA_DIR
    $oldMode = $env:ATELIER_DOCK_ONLY_TEST
    $oldRun = $env:ATELIER_DOCK_NATIVE_TEST
    $env:ATELIER_DATA_DIR = $testRoot
    $env:ATELIER_DOCK_ONLY_TEST = '1'
    $env:ATELIER_DOCK_NATIVE_TEST = '1'
    try {
        $fixture = Start-Process -FilePath $testExe -ArgumentList '--dock-app-fixture' -WindowStyle Hidden -PassThru
        $app = Start-Process -FilePath $testExe -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $testRoot 'stderr.log') -RedirectStandardOutput (Join-Path $testRoot 'stdout.log')
        $reportPath = Join-Path $testRoot 'dock-native-report.json'
        $deadline = (Get-Date).AddSeconds(40)
        while (!(Test-Path -LiteralPath $reportPath) -and (Get-Date) -lt $deadline -and !$app.HasExited) {
            Start-Sleep -Milliseconds 300
            $app.Refresh()
        }
        if (!(Test-Path -LiteralPath $reportPath)) { throw "No native test report. Diagnostics: $testRoot" }
        $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
        if (!$report.ok) { throw "Native test failed: $($report.error). Diagnostics: $testRoot" }
        $report.checks | ForEach-Object { 'PASS: ' + $_ }
        'Native report: ' + $reportPath
    } finally {
        Get-Process -Name Atelier -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $testExe } | Stop-Process
        $env:ATELIER_DATA_DIR = $oldData
        $env:ATELIER_DOCK_ONLY_TEST = $oldMode
        $env:ATELIER_DOCK_NATIVE_TEST = $oldRun
    }
} finally { Pop-Location }
