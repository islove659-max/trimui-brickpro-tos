# Build glremote runtime; run from armhf_gl.
# Chay: powershell -NoProfile -File build_runtime.ps1   (thu muc armhf_gl)
$ErrorActionPreference = "Continue"
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$cl = Join-Path $PSScriptRoot "client"; $sv = Join-Path $PSScriptRoot "server_rs"
$dist = Join-Path $PSScriptRoot "dist"; $stage = Join-Path $dist "stage"
New-Item -ItemType Directory -Force "$stage\lib32" | Out-Null
$rel = "target\armv7-unknown-linux-gnueabihf\release"
function Build($pkg, $extra, $out) {
  Push-Location $cl
  try {
    $o = & $cargo rustc --release -p $pkg -- @extra 2>&1 | Out-String
    if ($o -match "(?m)^error") { throw "build $pkg loi:`n$o" }
    $lib = "lib" + ($pkg -replace '-', '_') + ".so"
    Copy-Item "$rel\$lib" "$stage\lib32\$out" -Force
  } finally { Pop-Location }
}
Build glremote @("-C","link-arg=--version-script=glremote/exports.map") "libglremote.so"
Build alsa_shim @("-C","link-arg=--version-script=alsa_shim/alsa.map","-C","link-arg=-soname=libasound.so.2") "libasound.so.2"
Build sdl_deps_stub @("-C","link-arg=--version-script=sdl_deps_stub/xkb.map","-C","link-arg=-soname=libxkbcommon.so.0") "libxkbcommon.so.0"
Build sdl_deps_stub @("-C","link-arg=--version-script=sdl_deps_stub/pulse.map","-C","link-arg=-soname=libpulse.so.0") "libpulse.so.0"
Build sdl_deps_stub @("-C","link-arg=-soname=libsdl_deps_stub.so") "libsdl_deps_stub.so"
Build drmgbm_stub @("-C","link-arg=-soname=libdrm.so.2") "libdrm.so.2"
Push-Location $sv; try { powershell -NoProfile -File build.ps1 | Out-Null } finally { Pop-Location }
Copy-Item "$sv\target\aarch64-unknown-linux-musl\release\glserver" "$stage\glserver" -Force
New-Item -ItemType Directory -Force "$stage\lib32-sdl2" | Out-Null
Copy-Item "C:\Users\X\tools\sdl_armhf_dl\libSDL2-2.0.so.0" "$stage\lib32-sdl2\libSDL2-2.0.so.0" -Force
New-Item -ItemType Directory -Force "$stage\gmlibs" | Out-Null
Copy-Item (Join-Path $PSScriptRoot "gmlibs\*") "$stage\gmlibs\" -Force
Copy-Item (Join-Path $PSScriptRoot "runtime\glremote\lib32\libudev.so.1") "$stage\lib32\libudev.so.1" -Force
Copy-Item (Join-Path $PSScriptRoot "runtime\glremote_run") "$stage\glremote_run" -Force
New-Item -ItemType Directory -Force "$stage\licenses" | Out-Null
Copy-Item (Join-Path $PSScriptRoot "runtime\licenses\libudev-copyright.txt") "$stage\licenses\libudev-copyright.txt" -Force
Copy-Item (Join-Path $PSScriptRoot "runtime\trimui-controllerdb.txt") "$stage\trimui-controllerdb.txt" -Force
Copy-Item (Join-Path $PSScriptRoot "runtime\README.txt") "$stage\README.txt" -Force
python -I -X utf8 (Join-Path $PSScriptRoot "pack_runtime.py") $stage $dist
Write-Host "xong: $dist"

