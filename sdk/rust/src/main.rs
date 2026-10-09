//! hello-tos: app mẫu TOS. Kiểm tra fb (pan + vsync), evdev + ánh xạ phím, âm thanh (bíp/WAV),
//! chữ tiếng Việt, và đọc trạng thái hệ thống (âm lượng/tắt tiếng/độ sáng).
//! Thoát: MENU hoặc B (mặc định, theo bản đồ phím), hoặc SIGTERM/SIGINT.
//! Tham số: --frames N | --audio | --beep | --beeps | --wav <tệp> | --keymap <tệp>
mod audio;
mod fb;
mod input;
mod keymap;
mod sys;
mod sysinfo;
mod text;

use keymap::Action;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn on_signal(_: i32) { STOP.store(true, Ordering::SeqCst); }

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let max_frames: u64 = arg_value(&args, "--frames").and_then(|s| s.parse().ok()).unwrap_or(u64::MAX);
    unsafe { sys::signal(sys::SIGTERM, on_signal as *const () as usize); sys::signal(sys::SIGINT, on_signal as *const () as usize); }

    let mut fb = fb::Fb::open("/dev/fb0").map_err(|e| e.to_string())?;
    let mut pads = input::Pads::scan();
    let keymap = match arg_value(&args, "--keymap") { Some(p) => keymap::KeyMap::load(&p), None => keymap::KeyMap::default_map() };
    let mut text = text::Text::new(fb.channel_offsets())?;
    eprintln!("fb {}x{} stride={}B trang={} pads={}", fb.width, fb.height, fb.stride, fb.pages(), pads.count());

    let (w, h, sp) = (fb.width as usize, fb.height as usize, fb.stride_px());
    let bg = fb.rgb(10, 14, 30);
    let bars: Vec<u32> = [(230, 60, 60), (240, 180, 50), (80, 200, 100), (60, 140, 230), (160, 90, 220)]
        .iter().map(|&(r, g, b)| fb.rgb(r, g, b)).collect();
    let white = fb.rgb(255, 255, 255);

    // Âm thanh: tuỳ chọn, lỗi không làm app dừng (chỉ cảnh báo). KHÔNG đụng mixer hệ thống.
    let want_audio = args.iter().any(|a| matches!(a.as_str(), "--beep" | "--beeps" | "--audio" | "--wav"));
    let audio = if want_audio {
        match audio::Audio::open("PlaybackDmix") {
            Ok(a) => Some(a),
            Err(e) => { eprintln!("CẢNH BÁO âm thanh: {}", e); None }
        }
    } else { None };
    let wav = match arg_value(&args, "--wav") {
        Some(p) => match audio::load_wav(&p, 0.8) { Ok(s) => { eprintln!("WAV nạp: {} mẫu stereo 48k", s.len() / 2); Some(s) }, Err(e) => { eprintln!("CẢNH BÁO WAV: {}", e); None } },
        None => None,
    };
    if args.iter().any(|a| a == "--beep") {
        if let Some(a) = audio.as_ref() { a.play(audio::beep(880.0, 250, 0.25)); }
    }
    let mut audio = audio;

    let mut keys = Vec::new();
    let t0 = Instant::now();
    let mut frame: u64 = 0;
    let mut last_report = Instant::now();
    let mut worst = Duration::ZERO;
    let mut t_prev = Instant::now();
    let (mut x, mut dx) = (50i32, 9i32);
    let mut lit: u32 = 0; // bitmask nút đang giữ
    let mut sysstate = sysinfo::read();
    let mut last_actions = String::from("(chưa bấm)");

    while !STOP.load(Ordering::SeqCst) && frame < max_frames {
        keys.clear(); pads.poll(&mut keys);
        for k in &keys {
            let act = keymap.action(k.code);
            if k.down && !k.repeat {
                last_actions = format!("{:?} (mã {})", act.map(|a| format!("{:?}", a)).unwrap_or_else(|| "?".into()), k.code);
                match act {
                    Some(Action::Menu) | Some(Action::QuayLai) => STOP.store(true, Ordering::SeqCst),
                    Some(Action::XacNhan) => { if let Some(a) = audio.as_ref() { a.play(audio::beep(660.0, 200, 0.35)); } }
                    Some(Action::PhuX) => { if let (Some(a), Some(s)) = (audio.as_ref(), wav.as_ref()) { a.play(s.clone()); } }
                    _ => {}
                }
            }
            let bit = match act { Some(Action::XacNhan) => 0, Some(Action::PhuX) => 1, Some(Action::PhuY) => 2, Some(Action::Len) => 3, Some(Action::Xuong) => 4, Some(Action::Trai) => 5, Some(Action::Phai) => 6, _ => 31 };
            if bit < 31 { if k.down { lit |= 1 << bit } else { lit &= !(1 << bit) } }
        }
        if args.iter().any(|a| a == "--beeps") {
            if let Some(a) = audio.as_ref() {
                match frame { 60 => a.play(audio::beep(880.0, 400, 0.4)), 120 => a.play(audio::beep(660.0, 400, 0.4)), 180 => a.play(audio::beep(440.0, 400, 0.4)), _ => {} }
            }
        }
        if frame % 30 == 0 { sysstate = sysinfo::read(); }   // làm mới 2 lần/giây
        x += dx; if x < 0 || x > (w as i32 - 120) { dx = -dx; x += dx; }

        let buf = fb.back();
        for row in 0..h {
            let line = &mut buf[row * sp..row * sp + w];
            let band = (row * bars.len() / h).min(bars.len() - 1);
            for p in line.iter_mut() { *p = bg; }
            if row % 96 < 4 { for p in line.iter_mut() { *p = bars[band]; } }
        }
        for row in (h / 2 - 60)..(h / 2 + 60) { for col in x as usize..(x as usize + 120) { buf[row * sp + col] = white; } }
        for i in 0..7usize {
            let c = if lit & (1 << i) != 0 { bars[i % bars.len()] } else { bg + 0x00202020 };
            for row in 20..52 { for col in (20 + i * 44)..(20 + i * 44 + 36) { buf[row * sp + col] = c; } }
        }
        // Chữ tiếng Việt (có dấu) + trạng thái hệ thống
        text.draw(buf, sp, w, h, 20, 70, "TOS — Xin chào TrimUI Brick Pro", 40, (255, 255, 255));
        let sys_line = match sysstate {
            Some(s) => format!("Âm lượng {}/20 · Tắt tiếng: {} · Độ sáng {}", s.volume, if s.muted { "có" } else { "không" }, s.brightness),
            None => "Không đọc được trạng thái hệ thống".to_string(),
        };
        text.draw(buf, sp, w, h, 20, 125, &sys_line, 28, (180, 220, 255));
        text.draw(buf, sp, w, h, 20, h as i32 - 150, "A: phát bíp   X: phát WAV (--wav)   B hoặc MENU: thoát", 26, (200, 200, 200));
        text.draw(buf, sp, w, h, 20, h as i32 - 110, &format!("Phím vừa bấm: {}", last_actions), 26, (240, 200, 90));
        text.draw(buf, sp, w, h, 20, h as i32 - 70, "Ô vuông chạy ngang kiểm tra pan 2 trang + vsync", 22, (150, 150, 170));
        fb.flip();
        frame += 1;
        let now = Instant::now(); worst = worst.max(now - t_prev); t_prev = now;
        if last_report.elapsed() >= Duration::from_secs(2) {
            eprintln!("fps={:.1} khung_cham_nhat={:.1}ms vsync={}", frame as f64 / t0.elapsed().as_secs_f64(), worst.as_secs_f64() * 1000.0, fb.vsync_ok);
            last_report = Instant::now(); worst = Duration::ZERO;
        }
    }
    if let Some(a) = audio.as_mut() { eprintln!("aplay còn sống: {}", a.alive()); }
    eprintln!("xong: {} khung, {:.2}s, fps_tb={:.1}", frame, t0.elapsed().as_secs_f64(), frame as f64 / t0.elapsed().as_secs_f64());
    Ok(())
}

fn main() {
    if let Err(e) = run() { eprintln!("LỖI: {}", e); std::process::exit(1); }
}
