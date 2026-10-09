//! Khung SDL2 gia: chi de lien ket (ten ky hieu). Khong chay thuc; thu vien that nap luc chay tren may.
#![no_std]
use core::panic::PanicInfo;
#[panic_handler] fn panic(_: &PanicInfo) -> ! { loop {} }
macro_rules! stub { ($($n:ident),*) => { $( #[no_mangle] pub extern "C" fn $n() {} )* } }
stub!(SDL_Init, SDL_GetError, SDL_GL_SetAttribute, SDL_CreateWindow, SDL_GL_CreateContext, SDL_GL_SwapWindow,
      SDL_GL_SetSwapInterval, SDL_GL_GetProcAddress, SDL_PumpEvents, SDL_GL_DeleteContext, SDL_DestroyWindow, SDL_Quit);
