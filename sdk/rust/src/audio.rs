//! Âm thanh: luồng PCM liên tục (S16_LE, 48000Hz, 2 kênh) đẩy vào `aplay` (có sẵn trên Stock).
//! Lý do dùng luồng liên tục: ghi từng đoạn ngắn (vd 120ms) vào aplay không đủ để nó bắt đầu phát
//! (aplay đọc theo khối và chờ đủ ngưỡng bắt đầu). Luồng này luôn gửi mẫu (im lặng khi không có gì),
//! và trộn "giọng" (tiếng bíp...) vào ngay khi có yêu cầu. Không dùng `sh -c`.
//! Thiết bị mặc định `PlaybackDmix` (đã đo trên Brick Pro). Ống dẫn được thu nhỏ để giảm độ trễ.
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const RATE: u32 = 48_000;
const CHUNK_FRAMES: usize = 480; // 10ms

extern "C" {
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}
const F_SETPIPE_SZ: i32 = 1031;

struct Voice {
    samples: Vec<i16>, // L,R xen kẽ
    pos: usize,
}

pub struct Audio {
    child: Child,
    voices: Arc<Mutex<Vec<Voice>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Audio {
    pub fn open(device: &str) -> Result<Audio, String> {
        let mut child = Command::new("aplay")
            .args(["-q", "-D", device, "-t", "raw", "-f", "S16_LE", "-r", &RATE.to_string(), "-c", "2",
                   "-B", "80000", "-F", "20000", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("không chạy được aplay: {}", e))?;
        let mut stdin = child.stdin.take().ok_or("không có stdin")?;
        unsafe { fcntl(stdin.as_raw_fd(), F_SETPIPE_SZ, 4096i32); }   // ống nhỏ => độ trễ thấp
        let voices: Arc<Mutex<Vec<Voice>>> = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (v2, s2) = (voices.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            let mut buf = vec![0i16; CHUNK_FRAMES * 2];
            let mut bytes = vec![0u8; CHUNK_FRAMES * 4];
            while !s2.load(Ordering::SeqCst) {
                for s in buf.iter_mut() { *s = 0; }
                {
                    let mut vs = v2.lock().unwrap();
                    for v in vs.iter_mut() {
                        let n = (buf.len()).min(v.samples.len() - v.pos);
                        for i in 0..n {
                            buf[i] = buf[i].saturating_add(v.samples[v.pos + i]);
                        }
                        v.pos += n;
                    }
                    vs.retain(|v| v.pos < v.samples.len());
                }
                for (i, s) in buf.iter().enumerate() { bytes[i * 2..i * 2 + 2].copy_from_slice(&s.to_le_bytes()); }
                if stdin.write_all(&bytes).is_err() { break; }     // aplay đã thoát
            }
        });
        Ok(Audio { child, voices, stop, thread: Some(thread) })
    }

    /// Thêm một âm (mẫu L,R xen kẽ); phát ngay ở khối kế tiếp (độ trễ ~ vài chục ms).
    pub fn play(&self, samples: Vec<i16>) {
        self.voices.lock().unwrap().push(Voice { samples, pos: 0 });
    }

    /// aplay còn sống không (false = lỗi mở thiết bị hoặc đã thoát).
    pub fn alive(&mut self) -> bool { matches!(self.child.try_wait(), Ok(None)) }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.child.kill();                 // làm aplay thoát => luồng ghi thoát khỏi write
        let _ = self.child.wait();                 // tránh tiến trình zombie
        if let Some(t) = self.thread.take() { let _ = t.join(); }
    }
}

/// Tạo tiếng bíp hình sin (L=R), `ms` mili giây, có vào/ra mượt 5ms để không "bụp".
pub fn beep(freq: f32, ms: u32, volume: f32) -> Vec<i16> {
    let n = (RATE as u64 * ms as u64 / 1000) as usize;
    let ramp = (RATE as usize / 200).min(n / 2).max(1);
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let env = if i < ramp { i as f32 / ramp as f32 } else if i + ramp > n { (n - i) as f32 / ramp as f32 } else { 1.0 };
        let v = ((t * freq * std::f32::consts::TAU).sin() * volume * env * 32767.0) as i16;
        out.push(v); out.push(v);
    }
    out
}

/// Đọc WAV PCM 8/16/24/32-bit (mono hoặc stereo, tốc độ bất kỳ) -> mẫu S16 stereo 48000Hz (nội suy tuyến tính).
/// `gain` nhân biên độ (1.0 = giữ nguyên). Không hỗ trợ nén (ADPCM/MP3...).
pub fn load_wav(path: &str, gain: f32) -> Result<Vec<i16>, String> {
    let d = std::fs::read(path).map_err(|e| format!("không đọc được {}: {}", path, e))?;
    if d.len() < 44 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" { return Err("không phải WAV".into()); }
    let (mut ch, mut rate, mut bits, mut fmt) = (0u16, 0u32, 0u16, 0u16);
    let mut data: &[u8] = &[];
    let mut i = 12;
    while i + 8 <= d.len() {
        let id = &d[i..i + 4];
        let sz = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]) as usize;
        let body = &d[(i + 8).min(d.len())..(i + 8 + sz).min(d.len())];
        if id == b"fmt " && body.len() >= 16 {
            fmt = u16::from_le_bytes([body[0], body[1]]);
            ch = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" { data = body; }
        i += 8 + sz + (sz & 1);
    }
    if fmt != 1 && fmt != 0xFFFE { return Err(format!("định dạng WAV {} chưa hỗ trợ (chỉ PCM)", fmt)); }
    if ch == 0 || ch > 2 || rate == 0 { return Err("WAV thiếu/hỏng fmt".into()); }
    let bytes = (bits / 8) as usize;
    if !(1..=4).contains(&bytes) { return Err(format!("{} bit chưa hỗ trợ", bits)); }
    let frames = data.len() / (bytes * ch as usize);
    let sample = |f: usize, c: usize| -> f32 {
        let o = (f * ch as usize + c) * bytes;
        match bytes {
            1 => (data[o] as f32 - 128.0) / 128.0,
            2 => i16::from_le_bytes([data[o], data[o + 1]]) as f32 / 32768.0,
            3 => (((data[o] as i32) | ((data[o + 1] as i32) << 8) | ((data[o + 2] as i32) << 16)) << 8 >> 8) as f32 / 8388608.0,
            _ => i32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as f32 / 2147483648.0,
        }
    };
    let out_frames = (frames as u64 * RATE as u64 / rate as u64) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    for n in 0..out_frames {
        let pos = n as f64 * rate as f64 / RATE as f64;
        let (i0, fr) = (pos.floor() as usize, (pos - pos.floor()) as f32);
        let i1 = (i0 + 1).min(frames.saturating_sub(1));
        let get = |c: usize| { let c = c.min(ch as usize - 1); sample(i0.min(frames - 1), c) * (1.0 - fr) + sample(i1, c) * fr };
        for c in 0..2 { out.push(((get(c) * gain).clamp(-1.0, 1.0) * 32767.0) as i16); }
    }
    Ok(out)
}