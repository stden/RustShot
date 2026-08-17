[CmdletBinding(DefaultParameterSetName = "Evaluate")]
param(
    [Parameter(Mandatory = $true, ParameterSetName = "Evaluate")]
    [string]$CriterionRoot,

    [Parameter(ParameterSetName = "Evaluate")]
    [ValidateRange(0.0, 1.0)]
    [double]$RegressionThreshold = 0.10,

    [Parameter(Mandatory = $true, ParameterSetName = "SelfTest")]
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Budgets = @(
    # Steady-state cursor-move redraw. The budget is deliberately far below the
    # full-frame figure below: it is what fails if the overlay ever goes back to
    # recomposing the whole frame on every mouse move.
    [pscustomobject]@{
        Benchmark = "performance_gate/selection_overlay_redraw_1080p"
        MaximumNanoseconds = 200000.0
    }
    [pscustomobject]@{
        Benchmark = "performance_gate/selection_overlay_first_frame_1080p"
        MaximumNanoseconds = 2000000.0
    }
    [pscustomobject]@{
        Benchmark = "performance_gate/bgra_to_rgba_4k"
        MaximumNanoseconds = 4000000.0
    }
    [pscustomobject]@{
        Benchmark = "performance_gate/recording_frame_copy_1080p"
        MaximumNanoseconds = 2000000.0
    }
)

function Get-JsonNumber {
    param(
        [Parameter(Mandatory = $true)][string]$Json,
        [Parameter(Mandatory = $true)][string[]]$PropertyPath
    )

    try {
        $current = $Json | ConvertFrom-Json -AsHashtable
    } catch {
        throw "Invalid Criterion JSON: $($_.Exception.Message)"
    }
    foreach ($property in $PropertyPath) {
        if ($current -isnot [System.Collections.IDictionary] -or
            -not $current.Contains($property)) {
            throw "Criterion JSON is missing $($PropertyPath -join '.')"
        }
        $current = $current[$property]
    }
    try {
        return [double]$current
    } catch {
        throw "Criterion JSON field $($PropertyPath -join '.') is not a number"
    }
}

function Get-AbsoluteMeasurement {
    param(
        [Parameter(Mandatory = $true)][string]$Json,
        [Parameter(Mandatory = $true)][double]$BudgetNanoseconds
    )

    $estimate = Get-JsonNumber -Json $Json -PropertyPath @("median", "point_estimate")
    $upperBound = Get-JsonNumber -Json $Json -PropertyPath @(
        "median", "confidence_interval", "upper_bound"
    )
    [pscustomobject]@{
        EstimateNanoseconds = $estimate
        UpperBoundNanoseconds = $upperBound
        ExceedsBudget = $upperBound -gt $BudgetNanoseconds
    }
}

function Get-RelativeMeasurement {
    param(
        [Parameter(Mandatory = $true)][string]$Json,
        [Parameter(Mandatory = $true)][double]$Threshold
    )

    $estimate = Get-JsonNumber -Json $Json -PropertyPath @("median", "point_estimate")
    $lowerBound = Get-JsonNumber -Json $Json -PropertyPath @(
        "median", "confidence_interval", "lower_bound"
    )
    [pscustomobject]@{
        Estimate = $estimate
        LowerBound = $lowerBound
        ConfirmedRegression = $lowerBound -gt $Threshold
    }
}

function Get-BenchmarkEstimatePath {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Benchmark,
        [Parameter(Mandatory = $true)][string]$Baseline
    )

    $path = $Root
    foreach ($component in $Benchmark.Split("/")) {
        $path = Join-Path $path $component
    }
    Join-Path (Join-Path $path $Baseline) "estimates.json"
}

function Format-Duration {
    param([Parameter(Mandatory = $true)][double]$Nanoseconds)

    if ($Nanoseconds -ge 1000000.0) {
        return "{0:F3} ms" -f ($Nanoseconds / 1000000.0)
    }
    if ($Nanoseconds -ge 1000.0) {
        return "{0:F3} us" -f ($Nanoseconds / 1000.0)
    }
    return "{0:F3} ns" -f $Nanoseconds
}

function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)]$Expected,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

function Invoke-SelfTest {
    $absoluteJson = @'
{"median":{"confidence_interval":{"upper_bound":130.0},"point_estimate":120.0}}
'@
    $overBudget = Get-AbsoluteMeasurement -Json $absoluteJson -BudgetNanoseconds 100.0
    Assert-Equal $overBudget.EstimateNanoseconds 120.0 "Absolute estimate"
    Assert-Equal $overBudget.UpperBoundNanoseconds 130.0 "Absolute upper bound"
    Assert-Equal $overBudget.ExceedsBudget $true "Absolute budget decision"
    $withinBudget = Get-AbsoluteMeasurement -Json $absoluteJson -BudgetNanoseconds 140.0
    Assert-Equal $withinBudget.ExceedsBudget $false "Absolute passing decision"

    $confirmedJson = @'
{"median":{"confidence_interval":{"lower_bound":0.12},"point_estimate":0.16}}
'@
    $noisyJson = @'
{"median":{"confidence_interval":{"lower_bound":0.05},"point_estimate":0.16}}
'@
    $confirmed = Get-RelativeMeasurement -Json $confirmedJson -Threshold 0.10
    $noisy = Get-RelativeMeasurement -Json $noisyJson -Threshold 0.10
    Assert-Equal $confirmed.ConfirmedRegression $true "Confirmed regression decision"
    Assert-Equal $noisy.ConfirmedRegression $false "Noisy regression decision"

    $malformedFailed = $false
    try {
        Get-AbsoluteMeasurement -Json "{}" -BudgetNanoseconds 100.0 | Out-Null
    } catch {
        $malformedFailed = $true
    }
    Assert-Equal $malformedFailed $true "Malformed Criterion output"

    Write-Host "Benchmark guard self-tests passed."
}

if ($SelfTest) {
    Invoke-SelfTest
    return
}

$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add(
    "Performance gate (confirmed regression threshold: {0:F1}%)" -f
        ($RegressionThreshold * 100.0)
)
$failures = 0
$comparisonCount = 0

foreach ($budget in $Budgets) {
    $newPath = Get-BenchmarkEstimatePath -Root $CriterionRoot -Benchmark $budget.Benchmark -Baseline "new"
    if (-not (Test-Path -LiteralPath $newPath)) {
        throw "Missing Criterion result: $newPath"
    }
    $measurement = Get-AbsoluteMeasurement -Json (Get-Content -Raw -LiteralPath $newPath) -BudgetNanoseconds $budget.MaximumNanoseconds
    $status = if ($measurement.ExceedsBudget) {
        $failures++
        "FAIL"
    } else {
        "PASS"
    }
    $lines.Add(
        "$status $($budget.Benchmark): median " +
        "$(Format-Duration $measurement.EstimateNanoseconds), upper bound " +
        "$(Format-Duration $measurement.UpperBoundNanoseconds), budget " +
        "$(Format-Duration $budget.MaximumNanoseconds)"
    )

    $changePath = Get-BenchmarkEstimatePath -Root $CriterionRoot -Benchmark $budget.Benchmark -Baseline "change"
    if (Test-Path -LiteralPath $changePath) {
        $comparisonCount++
        $change = Get-RelativeMeasurement -Json (Get-Content -Raw -LiteralPath $changePath) -Threshold $RegressionThreshold
        $status = if ($change.ConfirmedRegression) {
            $failures++
            "FAIL"
        } else {
            "PASS"
        }
        $lines.Add(
            "$status $($budget.Benchmark): median change " +
            ("{0:+0.0;-0.0;0.0}%, lower confidence bound " -f
                ($change.Estimate * 100.0)) +
            ("{0:+0.0;-0.0;0.0}%" -f ($change.LowerBound * 100.0))
        )
    }
}

if ($comparisonCount -eq 0) {
    $lines.Add("INFO no base benchmark is available; absolute budgets were enforced")
}

foreach ($line in $lines) {
    Write-Host $line
}

if (Test-Path Env:GITHUB_STEP_SUMMARY) {
    $newline = [Environment]::NewLine
    $summary = "## Performance benchmark gate" + $newline + $newline +
        "~~~text" + $newline + ($lines -join $newline) + $newline + "~~~"
    Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Value $summary -Encoding utf8
}

if ($failures -gt 0) {
    throw "$failures benchmark performance check(s) failed."
}
