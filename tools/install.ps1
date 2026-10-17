# Install step for Windows (herdr-plugin.toml [[build]]). Fetches the prebuilt binary
# from the GitHub release that matches Cargo.toml's version, checks its SHA-256, and
# puts it where herdr runs it. Anything missing or wrong falls back to building from
# source with cargo. Only this install step uses the network; the stage never does.
Set-Location (Split-Path -Parent $PSScriptRoot)
$ver = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$name = "herdr-fantasy-x86_64-pc-windows-msvc.exe"
$base = "https://github.com/KantaKan/herdr-fantasy/releases/download/v$ver"
$bin = Join-Path $env:TEMP "$name.$PID"
$sum = "$bin.sha256"
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$name" -OutFile $bin -ErrorAction Stop
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$name.sha256" -OutFile $sum -ErrorAction Stop
    $want = ((Get-Content $sum -Raw) -split '\s+')[0]
    $got = (Get-FileHash $bin -Algorithm SHA256).Hash
    if ($want -and ($got -ieq $want)) {
        New-Item -ItemType Directory -Force target\release | Out-Null
        Move-Item -Force $bin target\release\herdr-fantasy.exe
        Remove-Item -Force $sum -ErrorAction SilentlyContinue
        Write-Output "herdr-fantasy: installed the prebuilt $name v$ver"
        exit 0
    }
    Write-Warning "herdr-fantasy: checksum mismatch for $name"
} catch {
    Write-Warning "herdr-fantasy: no prebuilt $name for v$ver ($($_.Exception.Message))"
}
Remove-Item -Force $bin, $sum -ErrorAction SilentlyContinue
Write-Warning "herdr-fantasy: building from source with cargo"
cargo build --release --locked
exit $LASTEXITCODE
