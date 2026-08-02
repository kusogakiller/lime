# Lime Performance Benchmark Runner
#
# Usage:  pwsh benchmarks/run_benchmarks.ps1   (or:  powershell -File ...)
#
# What it does (measurement-only, no guessing):
#   1. cargo build --workspace
#   2. cargo test  --workspace  (must stay 29/29)
#   3. For each benchmark: 5 warmup + 30 timed runs of
#      `lime run <citrus.toml> --emit-ll` with LIME_PROFILE=1.
#      Wall time = Measure-Command; stage times = parsed from stderr [profile].
#   4. Stats: mean / median / min / max / stddev  (wall + per stage).
#   5. Maps the 10 requested stages onto actually-measured timers.
#      Stages that do not exist in this compiler build (LLVM Optimization,
#      Object Generation, Link) are recorded as null with an explicit N/A note
#      -- numbers are NEVER fabricated.
#   6. Compares against the most recent prior history file: prints improvement
#      (%) and regression (%). If the overall wall median is >= 5% SLOWER than
#      the previous run, prints "PERFORMANCE REGRESSION".
#   7. Writes benchmarks/history/YYYY-MM-DD-<commit>.{json,csv,md}.
#   8. Prints a final report: changes, improved/worsened stages, overall
#      improvement, and the next bottleneck (largest measured stage).

$ErrorActionPreference = "Continue"

$Root = (Get-Location).Path
$Bin = Join-Path $Root "target\debug\lime.exe"
$HistDir = Join-Path $Root "benchmarks\history"
New-Item -ItemType Directory -Force -Path $HistDir | Out-Null

$WARMUP = 5
$RUNS   = 30

# Benchmark -> citrus.toml path (relative to repo root)
$Benchmarks = @{
    "small"                  = "benchmarks/programs/small/citrus.toml"
    "medium"                 = "benchmarks/programs/medium/citrus.toml"
    "large"                  = "benchmarks/programs/large/citrus.toml"
    "generic_heavy"          = "benchmarks/programs/generic_heavy/citrus.toml"
    "package_heavy"          = "benchmarks/programs/package_heavy/citrus.toml"
    "monomorphization_heavy" = "benchmarks/programs/monomorphization_heavy/citrus.toml"
}

function Get-Commit {
    $h = (git rev-parse --short HEAD 2>$null)
    if (-not $h) { $h = "unknown" }
    return $h
}

function Stats([double[]]$xs) {
    $n = $xs.Count
    $sorted = $xs | Sort-Object
    $mean = ($xs | Measure-Object -Average).Average
    $median = if ($n % 2 -eq 1) { $sorted[($n-1)/2] } else { ($sorted[$n/2-1] + $sorted[$n/2]) / 2 }
    $min = $sorted[0]; $max = $sorted[$n-1]
    $variance = ($xs | ForEach-Object { [math]::Pow($_ - $mean, 2) } | Measure-Object -Average).Average
    $sd = [math]::Sqrt($variance)
    return @{ mean=$mean; median=$median; min=$min; max=$max; stddev=$sd; n=$n }
}

function Mean([double[]]$xs) {
    if ($xs.Count -eq 0) { return $null }
    return ($xs | Measure-Object -Average).Average
}

# Parse [profile] lines from stderr text into a hashtable stage->total micros.
function Parse-Profile([string]$text) {
    $ht = @{}
    foreach ($line in $text -split "`n") {
        if ($line -match '\[profile\]\s+([A-Za-z_]+):\s+(\d+)\s+us') {
            $name = $Matches[1]; $us = [double]$Matches[2]
            if ($ht.ContainsKey($name)) { $ht[$name] += $us } else { $ht[$name] = $us }
        }
    }
    return $ht
}

# Map the 10 requested stages to measured timers. Returns ordered list of
# @{stage; us or $null; note}.
function Map-Stages($prof) {
    $map = @(
        @{ stage="Lexer"; keys=@("tokenize"); note="tokenize" },
        @{ stage="Parser"; keys=@("parse"); note="parse" },
        @{ stage="Package Resolver"; keys=@("pkg_parse","dep_graph"); note="pkg_parse + dep_graph" },
        @{ stage="Import Resolver"; keys=@("import_resolve","apply_to_defs"); note="import_resolve + apply_to_defs" },
        @{ stage="Type Checker"; keys=@("type_check_located","resolve_operators_defs","resolve_operators_stmts","check_interface_conformance"); note="type_check_located + operator/interface passes" },
        @{ stage="Monomorphization"; keys=@("monomorphize_all"); note="monomorphize_all" },
        @{ stage="LLVM IR Generation"; keys=@("codegen_ll"); note="codegen_ll (textual IR emit)" },
        @{ stage="LLVM Optimization"; keys=@(); note="N/A - not implemented in this build" },
        @{ stage="Object Generation"; keys=@(); note="N/A - not implemented in this build" },
        @{ stage="Link"; keys=@(); note="N/A - not implemented in this build" }
    )
    $out = @()
    foreach ($m in $map) {
        $total = $null
        foreach ($k in $m.keys) {
            if ($prof.ContainsKey($k)) {
                $total = if ($null -eq $total) { $prof[$k] } else { $total + $prof[$k] }
            }
        }
        $out += @{ stage=$m.stage; us=$total; note=$m.note }
    }
    return $out
}

# ---------- Step 1: build ----------
Write-Output "=== [1/4] cargo build --workspace ==="
$buildOut = cargo build --workspace 2>&1
if ($LASTEXITCODE -ne 0) { Write-Error "BUILD FAILED"; $buildOut | Select-Object -Last 20; exit 1 }
Write-Output "build OK"

# ---------- Step 2: tests (must be 29/29) ----------
Write-Output "=== [2/4] cargo test --workspace (must stay 29/29) ==="
$tmp = Join-Path $env:TEMP "lime_test.txt"
cmd /c "cargo test --workspace > `"$tmp`" 2>&1"
$testTxt = Get-Content $tmp -Raw
$unit = [regex]::Match($testTxt, 'test result: ok\. (\d+) passed')
$integ = [regex]::Match($testTxt, '(?s).*test result: ok\. (\d+) passed')
$unitN = [int]$unit.Groups[1].Value
$integN = [int]$integ.Groups[1].Value
$total = $unitN + $integN
Write-Output ("  unit passed = {0}, integration passed = {1}, total = {2}" -f $unitN, $integN, $total)
if ($total -ne 29) {
    Write-Error ("TEST COUNT REGRESSION: expected 29/29, got {0}" -f $total)
    Remove-Item $tmp; exit 1
}
Write-Output "  29/29 maintained"
Remove-Item $tmp

# ---------- Step 3: benchmark each ----------
Write-Output ("=== [3/4] benchmarking ({0} warmup + {1} runs each, LIME_PROFILE=1) ===" -f $WARMUP, $RUNS)
$results = @{}

foreach ($name in $Benchmarks.Keys) {
    $rel = $Benchmarks[$name]
    $argPath = $rel -replace '/', '\'
    $full = Join-Path $Root $argPath

    # warmup
    for ($w=1; $w -le $WARMUP; $w++) {
        $env:LIME_PROFILE = "1"
        cmd /c "`"$Bin`" run `"$argPath`" --emit-ll > NUL 2> NUL" | Out-Null
    }

    $wall = @()
    $stageAgg = @{}   # stage name -> list of per-run totals
    for ($r=1; $r -le $RUNS; $r++) {
        $env:LIME_PROFILE = "1"
        $sw = Measure-Command { cmd /c "`"$Bin`" run `"$argPath`" --emit-ll > NUL 2> `"$env:TEMP\lime_prof.txt`"" | Out-Null }
        $wall += $sw.TotalMilliseconds
        $prof = Parse-Profile (Get-Content "$env:TEMP\lime_prof.txt" -Raw)
        foreach ($s in (Map-Stages $prof)) {
            if ($null -ne $s.us) {
                if (-not $stageAgg.ContainsKey($s.stage)) { $stageAgg[$s.stage] = @() }
                $stageAgg[$s.stage] += $s.us
            }
        }
    }
    $env:LIME_PROFILE = $null
    Remove-Item "$env:TEMP\lime_prof.txt" -ErrorAction SilentlyContinue
    # clean emitted .ll artifacts
    $ll = Join-Path (Split-Path $full) ([System.IO.Path]::GetFileNameWithoutExtension($full) + ".ll")
    Remove-Item $ll -ErrorAction SilentlyContinue

    $wstats = Stats $wall
    $stageStats = @{}
    foreach ($s in $stageAgg.Keys) { $stageStats[$s] = Stats $stageAgg[$s] }

    $results[$name] = @{ wall=$wstats; stages=$stageStats }
    Write-Output ("  {0,-22} wall median={1,8:F2}ms mean={2,8:F2}ms (min={3:F2} max={4:F2} sd={5:F2})" -f `
        $name, $wstats.median, $wstats.mean, $wstats.min, $wstats.max, $wstats.stddev)
}

# ---------- Step 4: compare + save history ----------
Write-Output "=== [4/4] compare with previous run + save history ==="
$commit = Get-Commit
$timestamp = (Get-Date).ToString("yyyy-MM-dd")
$timecode = (Get-Date).ToString("HHmmss")
$stamp = "{0}-{1}-{2}" -f $timestamp, $commit, $timecode

$prevFile = $null
$prev = $null
$existing = Get-ChildItem $HistDir -Filter *.json | Sort-Object Name
if ($existing.Count -ge 1) {
    # most recent previous (excluding the one we may be about to write)
    $candidate = $existing | Where-Object { $_.BaseName -ne $stamp } | Select-Object -Last 1
    if ($candidate) {
        $prevFile = $candidate.FullName
        $prev = Get-Content $candidate.FullName -Raw | ConvertFrom-Json
    }
}

# Build the data object
$benchOut = @()
foreach ($name in $Benchmarks.Keys) {
    $r = $results[$name]
    $stagesOut = @()
    foreach ($sm in (Map-Stages @{})) {
        $st = $r.stages[$sm.stage]
        if ($st) {
            $stagesOut += @{ stage=$sm.stage; note=$sm.note; mean_us=[math]::Round($st.mean,2); median_us=[math]::Round($st.median,2); min_us=[math]::Round($st.min,2); max_us=[math]::Round($st.max,2); stddev_us=[math]::Round($st.stddev,2) }
        } else {
            $stagesOut += @{ stage=$sm.stage; note=$sm.note; mean_us=$null; median_us=$null; min_us=$null; max_us=$null; stddev_us=$null }
        }
    }
    $benchOut += @{
        name = $name
        path = $Benchmarks[$name]
        wall = @{
            mean_ms=[math]::Round($r.wall.mean,2); median_ms=[math]::Round($r.wall.median,2)
            min_ms=[math]::Round($r.wall.min,2); max_ms=[math]::Round($r.wall.max,2)
            stddev_ms=[math]::Round($r.wall.stddev,2); n=$r.wall.n
        }
        stages = $stagesOut
    }
}

$data = @{
    timestamp = $timestamp
    commit = $commit
    rustc = (rustc --version 2>$null)
    lime_version = "debug"
    warmup = $WARMUP
    runs = $RUNS
    benchmarks = $benchOut
}

# ---- comparison ----
$compare = $null
if ($prev) {
    $cmp = @()
    $overallPrevMedian = 0.0; $overallCurMedian = 0.0
    $nBench = 0
    foreach ($b in $benchOut) {
        $p = $prev.benchmarks | Where-Object { $_.name -eq $b.name }
        if ($p) {
            $curM = $b.wall.median_ms; $prevM = $p.wall.median_ms
            $pct = if ($prevM -ne 0) { ($curM - $prevM) / $prevM * 100 } else { 0 }
            $cmp += @{ name=$b.name; prev_median_ms=[math]::Round($prevM,2); cur_median_ms=[math]::Round($curM,2); delta_pct=[math]::Round($pct,2); status=($(if($pct -gt 5){"REGRESSION"}elseif($pct -lt -1){"IMPROVED"}else{"SAME"})) }
            $overallPrevMedian += $prevM; $overallCurMedian += $curM; $nBench++
        }
    }
    $overallDelta = if ($overallPrevMedian -ne 0) { ($overallCurMedian - $overallPrevMedian) / $overallPrevMedian * 100 } else { 0 }
    $regression = $overallDelta -ge 5
    $compare = @{ previous_file=(Split-Path $prevFile -Leaf); per_benchmark=$cmp; overall_prev_median_ms=[math]::Round($overallPrevMedian,2); overall_cur_median_ms=[math]::Round($overallCurMedian,2); overall_delta_pct=[math]::Round($overallDelta,2); regression_flag=$regression }
    $data.compare = $compare
}

# ---- write JSON ----
$jsonPath = Join-Path $HistDir ($stamp + ".json")
$data | ConvertTo-Json -Depth 10 | ForEach-Object { [System.IO.File]::WriteAllText($jsonPath, $_) }

# ---- write CSV ----
$csvPath = Join-Path $HistDir ($stamp + ".csv")
$csvLines = @()
$csvLines += "benchmark,metric,mean,median,min,max,stddev,unit"
foreach ($b in $benchOut) {
    $w = $b.wall
    $csvLines += ("{0},wall,{1},{2},{3},{4},{5},ms" -f $b.name, $w.mean_ms, $w.median_ms, $w.min_ms, $w.max_ms, $w.stddev_ms)
    foreach ($s in $b.stages) {
        if ($null -ne $s.median_us) {
            $csvLines += ("{0},stage:{1},{2},{3},{4},{5},{6},us" -f $b.name, $s.stage, $s.mean_us, $s.median_us, $s.min_us, $s.max_us, $s.stddev_us)
        } else {
            $csvLines += ("{0},stage:{1},NA,NA,NA,NA,NA,us" -f $b.name, $s.stage)
        }
    }
}
$csvLines | ForEach-Object { [System.IO.File]::WriteAllText($csvPath, ($csvLines -join "`n")) }

# ---- write Markdown ----
$mdPath = Join-Path $HistDir ($stamp + ".md")
$md = @()
$md += "# Lime Performance Benchmark - $timestamp"
$md += ""
$md += "**commit:** ``$commit``  |  **warmup:** $WARMUP  |  **runs:** $RUNS"
$md += ""
$md += "## Wall-clock latency (``lime run --emit-ll``, median of $RUNS runs)"
$md += ""
$md += "| Benchmark | mean (ms) | median (ms) | min (ms) | max (ms) | stddev (ms) |"
$md += "|-----------|-----------|-------------|----------|----------|-------------|"
foreach ($b in $benchOut) {
    $w = $b.wall
    $md += ("| {0} | {1} | {2} | {3} | {4} | {5} |" -f $b.name, $w.mean_ms, $w.median_ms, $w.min_ms, $w.max_ms, $w.stddev_ms)
}
$md += ""
$md += "## Stage breakdown (median us, measured via LIME_PROFILE=1)"
$md += ""
foreach ($b in $benchOut) {
    $md += ("### {0}" -f $b.name)
    $md += ""
    $md += "| Stage | median (us) | mean (us) | note |"
    $md += "|-------|-------------|-----------|------|"
    foreach ($s in $b.stages) {
        $mu = if ($null -ne $s.median_us) { $s.median_us } else { "N/A" }
        $me = if ($null -ne $s.mean_us) { $s.mean_us } else { "N/A" }
        $md += ("| {0} | {1} | {2} | {3} |" -f $s.stage, $mu, $me, $s.note)
    }
    $md += ""
}
if ($compare) {
    $md += "## Comparison vs previous run ($($compare.previous_file))"
    $md += ""
    $md += ("**Overall wall median:** {0} ms -> {1} ms  (**{2}%**)  {3}" -f `
        $compare.overall_prev_median_ms, $compare.overall_cur_median_ms, $compare.overall_delta_pct, `
        $(if($compare.regression_flag){"[WARN] PERFORMANCE REGRESSION"}else{"[OK] no regression"}))
    $md += ""
    $md += "| Benchmark | prev median (ms) | cur median (ms) | delta % | status |"
    $md += "|-----------|------------------|----------------|---------|--------|"
    foreach ($c in $compare.per_benchmark) {
        $md += ("| {0} | {1} | {2} | {3} | {4} |" -f $c.name, $c.prev_median_ms, $c.cur_median_ms, $c.delta_pct, $c.status)
    }
    $md += ""
}
$md | ForEach-Object { [System.IO.File]::WriteAllText($mdPath, ($md -join "`n")) }

# ---------- Final report to stdout ----------
Write-Output ""
Write-Output "=================== FINAL REPORT ==================="
Write-Output ("Timestamp : {0}   Commit: {1}" -f $timestamp, $commit)
Write-Output ("Tests     : 29/29 maintained")
Write-Output ("Artifacts :")
Write-Output ("  $jsonPath")
Write-Output ("  $csvPath")
Write-Output ("  $mdPath")

if ($compare) {
    Write-Output ""
    Write-Output ("Comparison vs previous: {0}" -f $compare.previous_file)
    Write-Output ("  Overall wall median: {0} ms -> {1} ms  ({2}%)" -f `
        $compare.overall_prev_median_ms, $compare.overall_cur_median_ms, $compare.overall_delta_pct)
    if ($compare.regression_flag) {
        Write-Output "  *** PERFORMANCE REGRESSION: >= 5% slower overall ***"
    }
    # improved / worsened stages across all benchmarks (compare median us)
    $improved = @(); $worsened = @()
    foreach ($b in $benchOut) {
        $p = $prev.benchmarks | Where-Object { $_.name -eq $b.name }
        if (-not $p) { continue }
        foreach ($s in $b.stages) {
            $ps = $p.stages | Where-Object { $_.stage -eq $s.stage }
            if ($null -ne $s.median_us -and $ps -and $null -ne $ps.median_us -and $ps.median_us -ne 0) {
                $dpct = ($s.median_us - $ps.median_us) / $ps.median_us * 100
                if ($dpct -le -1) { $improved += ("{0}/{1}: {2}->{3}us ({4:F1}%)" -f $b.name, $s.stage, $ps.median_us, $s.median_us, $dpct) }
                elseif ($dpct -ge 5) { $worsened += ("{0}/{1}: {2}->{3}us (+{4:F1}%)" -f $b.name, $s.stage, $ps.median_us, $s.median_us, $dpct) }
            }
        }
    }
    Write-Output ""
    Write-Output ("Improved stages ({0}):" -f $improved.Count)
    $improved | ForEach-Object { "    + $_" }
    Write-Output ("Worsened stages ({0}):" -f $worsened.Count)
    $worsened | ForEach-Object { "    - $_" }

    # next bottleneck = largest median stage across all benchmarks this run
    $bottleneck = $null
    foreach ($b in $benchOut) {
        foreach ($s in $b.stages) {
            if ($null -ne $s.median_us -and ($null -eq $bottleneck -or $s.median_us -gt $bottleneck.us)) {
                $bottleneck = @{ name=$b.name; stage=$s.stage; us=$s.median_us }
            }
        }
    }
    if ($bottleneck) {
        Write-Output ""
        Write-Output ("Next bottleneck (largest measured stage): {0} / {1} = {2} us (median)" -f $bottleneck.name, $bottleneck.stage, $bottleneck.us)
    }
} else {
    Write-Output ""
    Write-Output "No previous history found - this is the BASELINE run."
    $bottleneck = $null
    foreach ($b in $benchOut) {
        foreach ($s in $b.stages) {
            if ($null -ne $s.median_us -and ($null -eq $bottleneck -or $s.median_us -gt $bottleneck.us)) {
                $bottleneck = @{ name=$b.name; stage=$s.stage; us=$s.median_us }
            }
        }
    }
    if ($bottleneck) {
        Write-Output ("Next bottleneck (largest measured stage): {0} / {1} = {2} us (median)" -f $bottleneck.name, $bottleneck.stage, $bottleneck.us)
    }
}
Write-Output "==================================================="
