# convert-gifs.ps1 — turn Monamoji (or any) GIFs in assets/source/ into
# assets/frames/<state>/NNNN.png frame sequences the engine auto-loads.
#
# Usage (this box has Windows PowerShell — use `powershell`, not `pwsh`):
#   powershell ./scripts/convert-gifs.ps1                # convert all recognized GIFs
#   powershell ./scripts/convert-gifs.ps1 happy idle     # only these states
#   powershell ./scripts/convert-gifs.ps1 -Size 80       # scale to 80x80 (default 64)
#   powershell ./scripts/convert-gifs.ps1 -Fps 12        # cap frames-per-second (optional)
#
# Source files are matched two ways:
#   1. Named directly after a state:        idle.gif, working.gif, happy.gif ...
#   2. Named after a known Monamoji mood:   monamoji-eye-rolls.gif -> idle, etc.
# Unknown names are skipped (with a note) so you can stage extras safely.
#
# Requires ffmpeg on PATH (confirmed: ffmpeg 8.1 via winget).

[CmdletBinding()]
param(
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]] $States,
    [int] $Size = 64,
    [int] $Fps = 0
)

$ErrorActionPreference = 'Stop'

# Canonical pet states (must match PetState::dir_name in src/state.rs).
$validStates = @(
    'idle', 'working', 'thinking', 'happy', 'celebrate', 'mindblown',
    'sleep', 'oops', 'error', 'rage', 'scared', 'sick'
)

# Monamoji GIF stem -> pet state. Edit here if you add/rename source GIFs.
$monamojiMap = @{
    'monamoji-eye-rolls'    = 'idle'
    'monamoji-typing'       = 'working'
    'monamoji-funny'        = 'thinking'
    'monamoji-love-hearts'  = 'happy'
    'monamoji-yay'          = 'celebrate'
    'monamoji-mind-blown'   = 'mindblown'
    'monamoji-zzz'          = 'sleep'
    'monamoji-oops-mistake' = 'oops'
    'monamoji-angry'        = 'error'
    'monamoji-rage'         = 'rage'
    'monamoji-oooh-scared'  = 'scared'
    'monamoji-vomits'       = 'sick'
}

$validExts = @('.gif', '.webp', '.apng', '.png')

# Resolve paths relative to the crate root (parent of this script's dir).
$root      = Split-Path -Parent $PSScriptRoot
$srcDir    = Join-Path $root 'assets/source'
$framesDir = Join-Path $root 'assets/frames'

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    throw "ffmpeg not found on PATH."
}
if (-not (Test-Path $srcDir)) {
    throw "Source dir not found: $srcDir"
}

$sources = Get-ChildItem -File $srcDir | Where-Object { $validExts -contains $_.Extension.ToLower() }
if (-not $sources) {
    Write-Host "No source GIFs in $srcDir. Drop e.g. monamoji-yay.gif there first." -ForegroundColor Yellow
    return
}

$wanted = if ($States) { $States } else { $validStates }
$did = 0

foreach ($f in $sources) {
    $stem = $f.BaseName.ToLower()

    # Resolve to a state: direct state name, else Monamoji map.
    $state = $null
    if ($validStates -contains $stem) {
        $state = $stem
    } elseif ($monamojiMap.ContainsKey($stem)) {
        $state = $monamojiMap[$stem]
    }

    if (-not $state) {
        Write-Host "skip  $($f.Name)  (no state mapping)" -ForegroundColor DarkYellow
        continue
    }
    if ($wanted -notcontains $state) { continue }

    $outDir = Join-Path $framesDir $state
    if (Test-Path $outDir) {
        Get-ChildItem -File $outDir -Filter *.png -ErrorAction SilentlyContinue | Remove-Item -Force
    } else {
        New-Item -ItemType Directory -Path $outDir | Out-Null
    }

    $vf = "scale=$($Size):$($Size):flags=lanczos:force_original_aspect_ratio=decrease,pad=$($Size):$($Size):(ow-iw)/2:(oh-ih)/2:color=0x00000000"
    if ($Fps -gt 0) { $vf = "fps=$Fps,$vf" }

    $outPattern = Join-Path $outDir '%04d.png'

    Write-Host "build $($f.Name)  ->  assets/frames/$state/  (${Size}px)" -ForegroundColor Cyan
    & ffmpeg -loglevel error -y -i $f.FullName -vf $vf $outPattern
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg failed on $($f.Name) (exit $LASTEXITCODE)"
    }

    $n = (Get-ChildItem -File $outDir -Filter *.png).Count
    Write-Host "   wrote $n frame(s)  ($stem -> $state)" -ForegroundColor Green
    $did++
}

if ($did -eq 0) {
    Write-Host "Nothing converted. Check source names against the Monamoji map." -ForegroundColor Yellow
} else {
    Write-Host "Done. $did state(s) converted. Run: cargo run" -ForegroundColor Green
}
