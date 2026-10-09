param(
 [Parameter(Mandatory=$true)][string]$Cli,
 [Parameter(Mandatory=$true)][string]$Original,
 [string]$Output = (Join-Path $PSScriptRoot "chip-music.droid")
)
$ErrorActionPreference="Stop"
if ((Get-FileHash -LiteralPath $Original -Algorithm SHA256).Hash.ToLower() -ne "9ab576aba53742c86135d7174e016c9905298007fde3e608156911fb73676ede") { throw "Game goc khong dung phien ban; dung lai de tranh patch nham." }
$patchArgs=@("replace",$Original,"-o",$Output)
Get-ChildItem (Join-Path $PSScriptRoot "gml") -Filter "*.gml" | ForEach-Object { $patchArgs += "-c"; $patchArgs += $_.BaseName+"="+$_.FullName }
& $Cli @patchArgs
if ($LASTEXITCODE -ne 0) { throw "UTMT compile failed" }
Get-FileHash -LiteralPath $Output
