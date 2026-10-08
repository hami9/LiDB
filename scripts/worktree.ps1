# PowerShell 5.1+ wrapper for the platform-independent Python worktree manager.
param([Parameter(ValueFromRemainingArguments = $true)][string[]] $CommandArgs)

$ErrorActionPreference = "Stop"
$ScriptPath = Join-Path $PSScriptRoot "worktrees.py"
$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    $Python = Get-Command py -ErrorAction SilentlyContinue
    if (-not $Python) { throw "Python 3 is required to use LiDB worktrees." }
    & $Python.Source -3 $ScriptPath @CommandArgs
} else {
    & $Python.Source $ScriptPath @CommandArgs
}
exit $LASTEXITCODE
