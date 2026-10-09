//! Ánh xạ phím theo cấu hình: hành động logic -> mã phím evdev. Có mặc định đúng nhãn in trên máy.
//! Tệp cấu hình (tuỳ chọn), mỗi dòng `hanh_dong=ma`, ví dụ:
//!     xac_nhan=305
//!     quay_lai=304
//! Dòng bắt đầu bằng # là chú thích. Hành động: xac_nhan, quay_lai, phu_x, phu_y, len, xuong, trai, phai, menu, bat_dau, chon.
use crate::input;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action { XacNhan, QuayLai, PhuX, PhuY, Len, Xuong, Trai, Phai, Menu, BatDau, Chon }

pub struct KeyMap { map: HashMap<u16, Action> }

fn name_to_action(s: &str) -> Option<Action> {
    Some(match s {
        "xac_nhan" => Action::XacNhan, "quay_lai" => Action::QuayLai, "phu_x" => Action::PhuX, "phu_y" => Action::PhuY,
        "len" => Action::Len, "xuong" => Action::Xuong, "trai" => Action::Trai, "phai" => Action::Phai,
        "menu" => Action::Menu, "bat_dau" => Action::BatDau, "chon" => Action::Chon,
        _ => return None,
    })
}

impl KeyMap {
    /// Mặc định theo quy ước UI của dự án: A xác nhận, B huỷ, X/Y phụ, D-pad, MENU thoát, START/SELECT.
    pub fn default_map() -> KeyMap {
        let mut m = HashMap::new();
        m.insert(input::BTN_A, Action::XacNhan);
        m.insert(input::BTN_B, Action::QuayLai);
        m.insert(input::BTN_X, Action::PhuX);
        m.insert(input::BTN_Y, Action::PhuY);
        m.insert(input::KEY_UP, Action::Len);
        m.insert(input::KEY_DOWN, Action::Xuong);
        m.insert(input::KEY_LEFT, Action::Trai);
        m.insert(input::KEY_RIGHT, Action::Phai);
        m.insert(input::BTN_MENU, Action::Menu);
        m.insert(315, Action::BatDau);
        m.insert(314, Action::Chon);
        KeyMap { map: m }
    }

    /// Nạp từ tệp; mục trong tệp THAY mặc định của đúng hành động đó. Lỗi đọc => dùng mặc định.
    pub fn load(path: &str) -> KeyMap {
        let mut km = KeyMap::default_map();
        let Ok(text) = std::fs::read_to_string(path) else { return km };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((k, v)) = line.split_once('=') {
                if let (Some(a), Ok(code)) = (name_to_action(k.trim()), v.trim().parse::<u16>()) {
                    km.map.retain(|_, act| *act != a);
                    km.map.insert(code, a);
                }
            }
        }
        km
    }

    pub fn action(&self, code: u16) -> Option<Action> { self.map.get(&code).copied() }
}
