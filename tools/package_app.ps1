# Đóng gói một app thành 4 zip (stock, knulli, spruce, nextui) từ thư mục có app.json.
# Dùng: .\package_app.ps1 -AppDir ..\apps_mau\hello_shell -Spec ..\sdk\spec\app.json -Out ..\out\pkg
param(
  [Parameter(Mandatory)][string]$AppDir,
  [Parameter(Mandatory)][string]$Spec,
  [string]$Out = "$PSScriptRoot\..\out\pkg"
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$enc = New-Object Text.UTF8Encoding $false
$app = Get-Content $Spec -Raw -Encoding UTF8 | ConvertFrom-Json
$id = $app.id; $name = $app.name
$sdk = Join-Path $PSScriptRoot '..\sdk\shell'
New-Item -ItemType Directory -Force $Out | Out-Null
$work = Join-Path $env:TEMP "tos_pkg_$id"
if (Test-Path $work) { Remove-Item -Recurse -Force $work }

function Put([string]$path, [string]$text) {
  New-Item -ItemType Directory -Force (Split-Path $path) | Out-Null
  [IO.File]::WriteAllText($path, $text.Replace("`r",""), $enc)
}
function CopyApp([string]$dest) {
  New-Item -ItemType Directory -Force "$dest\lib" | Out-Null
  Copy-Item "$AppDir\*" $dest -Recurse -Force
  Copy-Item "$sdk\tos_env.sh" "$dest\lib\tos_env.sh" -Force
  foreach ($f in Get-ChildItem $dest -Recurse -Include *.sh) {   # ép LF
    [IO.File]::WriteAllText($f.FullName, [IO.File]::ReadAllText($f.FullName).Replace("`r",""), $enc)
  }
}
$cfg = @{ label=$name; icon='icon.png'; iconsel='icon.png'; icontop='icon.png'; launch='launch.sh'; description=$app.description } | ConvertTo-Json
$pak = @{ name=$name; version=$app.version; type='TOOL'; platforms=@('tg5040','tg5050'); launch='launch.sh' } | ConvertTo-Json

# 1. Stock / CrossMix: Apps/<id>/
CopyApp "$work\stock\Apps\$id"; Put "$work\stock\Apps\$id\config.json" $cfg
# 2. Knulli: <Name>.sh ở gốc ports + thư mục app
CopyApp "$work\knulli\$id"
Put "$work\knulli\$name.sh" "#!/bin/sh`ncd `"`$(dirname `"`$0`")/$id`" || exit 1`nexec sh ./launch.sh`n"
# 3. SpruceOS: App/<name>/ (A hoa, số ít)
CopyApp "$work\spruce\App\$name"; Put "$work\spruce\App\$name\config.json" $cfg
# 4. NextUI: Tools/tg5040/<name>.pak/ (icon bắt buộc default.png)
CopyApp "$work\nextui\Tools\tg5040\$name.pak"; Put "$work\nextui\Tools\tg5040\$name.pak\pak.json" $pak
if (Test-Path "$AppDir\icon.png") { Copy-Item "$AppDir\icon.png" "$work\nextui\Tools\tg5040\$name.pak\default.png" -Force }

foreach ($os in 'stock','knulli','spruce','nextui') {
  $zip = Join-Path $Out "$id-$($app.version)-$os.zip"
  if (Test-Path $zip) { Remove-Item $zip }
  # Tu ghi zip de ten muc dung dau gach cheo xuoi (CreateFromDirectory cua PS 5.1 dung gach nguoc, hong khi giai nen tren Linux)
  $root = (Resolve-Path "$work\$os").Path.TrimEnd([char]92)
  $fs = [IO.File]::Open($zip, 'Create'); $za = New-Object IO.Compression.ZipArchive($fs, 'Create')
  foreach ($file in Get-ChildItem $root -Recurse -File) {
    $rel = $file.FullName.Substring($root.Length + 1).Replace([char]92, [char]47)
    [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($za, $file.FullName, $rel, 'Optimal')
  }
  $za.Dispose(); $fs.Dispose()
  "{0}  {1:N0} byte" -f $zip, (Get-Item $zip).Length
}
Remove-Item -Recurse -Force $work
