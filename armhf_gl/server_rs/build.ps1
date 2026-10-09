# Build glserver (aarch64, nhap SDL2 luc chay). Chay trong thu muc nay.
$ErrorActionPreference = "Stop"
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo rustc --release -p sdl_stub -- -C link-arg=-soname=libSDL2-2.0.so.0
Copy-Item target\aarch64-unknown-linux-musl\release\libsdl_stub.so stub\libSDL2-2.0.so -Force
& $cargo build --release -p glserver
Get-Item target\aarch64-unknown-linux-musl\release\glserver | Select Name, Length
