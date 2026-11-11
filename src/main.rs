#![windows_subsystem = "windows"]
//! IP Changer — Wi-Fi 어댑터의 IPv4를 고정 IP / DHCP로 바꾸는 도구.
//!
//! 외부 크레이트 없이 Windows 11에 기본으로 들어 있는 것만 쓴다.
//! - 화면: user32 / gdi32 / comctl32 (IP 주소 입력칸은 Windows 내장 SysIPAddress32)
//! - 네트워크 설정: Windows PowerShell
//! - 관리자 권한: exe 매니페스트(requireAdministrator)
//!
//! 화면은 FHD(100% 배율) 기준 픽셀 크기로 고정해서 그린다. 배율이 다른 모니터로
//! 옮겨도 크기가 바뀌지 않는다.

mod config;
mod json;
mod net;
mod win32;

use config::{normalize_subnet, validate_profile_name, Profile, Settings, Store};
use net::{
    apply_settings, ipv4_to_u32, list_adapters, parse_subnet, prefix_to_mask, truncate_chars,
    validate_static, AdapterInfo,
};
use std::cell::RefCell;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use win32::*;

// ------------------------------------------------------------
// 다국어
// ------------------------------------------------------------
fn tr<'a>(lang: &str, key: &'a str) -> &'a str {
    if lang == "en" {
        match key {
            "title" => "IP Changer",
            "refresh" => "Refresh",
            "mode_static" => "Manual (static IP)",
            "mode_dhcp" => "Automatic (DHCP)",
            "dhcp_note" => "The IP address and DNS servers are assigned automatically by the router.",
            "ip" => "IP address",
            "subnet" => "Subnet mask",
            "gateway" => "Gateway",
            "dns1" => "Preferred DNS",
            "dns2" => "Alternate DNS",
            "dns" => "DNS",
            "apply" => "Apply",
            "status_select_adapter" => "Please select an adapter.",
            "status_adapter_missing" => "Adapter not found. Please refresh the list.",
            "status_applying" => "Applying...",
            "status_loading" => "Reading network adapters...",
            "adapter_status_disconnected" => "Disconnected",
            "adapter_status_down" => "Disabled",
            "current_none" => "No IPv4 address",
            "current_dhcp" => "DHCP",
            "current_static" => "Static",
            "status_invalid_ip" => "Please enter a valid IP address.",
            "status_invalid_prefix" => "Enter a subnet mask such as 255.255.255.0.",
            "status_invalid_gateway" => "Gateway format is invalid or outside the subnet.",
            "status_invalid_dns1" => "Preferred DNS format is invalid.",
            "status_invalid_dns2" => "Alternate DNS format is invalid.",
            "status_static_done" => "Static IP configured.",
            "status_dhcp_done" => "Switched to DHCP.",
            "status_error" => "Error: {}",
            "status_save_failed" => "could not write settings.json",
            "lang_ko" => "한국어",
            "lang_en" => "English",
            "menu_save" => "Save settings",
            "menu_open_folder" => "Open settings folder",
            "profile" => "Profile",
            "profile_pick" => "Select a profile",
            "profile_save" => "Save",
            "profile_new" => "New",
            "profile_delete" => "Delete",
            "profile_name" => "Profile name",
            "profile_create" => "Create",
            "profile_default_name" => "Profile {}",
            "profile_delete_confirm" => "Delete profile '{}'?",
            "cancel" => "Cancel",
            "status_profile_loaded" => "Loaded '{}'. Press Apply to use it.",
            "status_profile_saved" => "Saved profile '{}'.",
            "status_profile_deleted" => "Deleted profile '{}'.",
            "status_profile_exists" => "A profile with that name already exists.",
            "profile_limit" => "You can create up to 3 profiles. Delete one first.",
            "profile_name_empty" => "Please enter a profile name.",
            "profile_name_too_long" => "Profile names must be 40 characters or fewer.",
            "profile_name_invalid" => "The profile name contains invalid characters.",
            "status_saved" => "Settings saved.",
            "status_adapter_list_failed" => "Could not read the adapter list.",
            "adapter_none" => "No Wi-Fi adapter found",
            _ => key,
        }
    } else {
        match key {
            "title" => "IP Changer",
            "refresh" => "새로고침",
            "mode_static" => "수동 (고정 IP)",
            "mode_dhcp" => "자동 (DHCP)",
            "dhcp_note" => "IP 주소와 DNS 서버를 공유기에서 자동으로 받아옵니다.",
            "ip" => "IP 주소",
            "subnet" => "서브넷 마스크",
            "gateway" => "게이트웨이",
            "dns1" => "기본 DNS",
            "dns2" => "보조 DNS",
            "dns" => "DNS",
            "apply" => "적용",
            "status_select_adapter" => "어댑터를 선택하세요.",
            "status_adapter_missing" => "어댑터를 찾을 수 없습니다. 목록을 새로고침하세요.",
            "status_applying" => "적용 중...",
            "status_loading" => "네트워크 어댑터를 읽는 중...",
            "adapter_status_disconnected" => "연결 안 됨",
            "adapter_status_down" => "사용 안 함",
            "current_none" => "IPv4 주소 없음",
            "current_dhcp" => "DHCP",
            "current_static" => "고정",
            "status_invalid_ip" => "올바른 IP 주소를 입력하세요.",
            "status_invalid_prefix" => "서브넷 마스크를 255.255.255.0 형식으로 입력하세요.",
            "status_invalid_gateway" => "게이트웨이 형식이 잘못되었거나 서브넷과 맞지 않습니다.",
            "status_invalid_dns1" => "기본 DNS 형식이 잘못되었습니다.",
            "status_invalid_dns2" => "보조 DNS 형식이 잘못되었습니다.",
            "status_static_done" => "고정 IP를 적용했습니다.",
            "status_dhcp_done" => "DHCP로 전환했습니다.",
            "status_error" => "오류: {}",
            "status_save_failed" => "settings.json에 저장하지 못했습니다",
            "lang_ko" => "한국어",
            "lang_en" => "English",
            "menu_save" => "설정 저장",
            "menu_open_folder" => "설정 폴더 열기",
            "profile" => "프로필",
            "profile_pick" => "프로필 선택",
            "profile_save" => "저장",
            "profile_new" => "새 프로필",
            "profile_delete" => "삭제",
            "profile_name" => "프로필 이름",
            "profile_create" => "만들기",
            "profile_default_name" => "프로필 {}",
            "profile_delete_confirm" => "'{}' 프로필을 삭제할까요?",
            "cancel" => "취소",
            "status_profile_loaded" => "'{}' 프로필을 불러왔습니다. 적용을 누르면 반영됩니다.",
            "status_profile_saved" => "'{}' 프로필을 저장했습니다.",
            "status_profile_deleted" => "'{}' 프로필을 삭제했습니다.",
            "status_profile_exists" => "같은 이름의 프로필이 이미 있습니다.",
            "profile_limit" => "프로필은 최대 3개까지 만들 수 있습니다. 먼저 하나를 삭제하세요.",
            "profile_name_empty" => "프로필 이름을 입력하세요.",
            "profile_name_too_long" => "프로필 이름은 40자 이하여야 합니다.",
            "profile_name_invalid" => "프로필 이름에 쓸 수 없는 문자가 있습니다.",
            "status_saved" => "설정을 저장했습니다.",
            "status_adapter_list_failed" => "어댑터 목록을 가져오지 못했습니다.",
            "adapter_none" => "Wi-Fi 어댑터가 없습니다",
            _ => key,
        }
    }
}

// ------------------------------------------------------------
// 색 / 크기 (모두 100% 배율 기준 픽셀)
// ------------------------------------------------------------
const PANEL_BG: u32 = rgb(245, 245, 247);
const CARD_BG: u32 = rgb(255, 255, 255);
const CARD_STROKE: u32 = rgb(228, 228, 233);
const SEGMENT_BG: u32 = rgb(236, 236, 240);
const BUTTON_STROKE: u32 = rgb(210, 210, 217);
const PRESSED_BG: u32 = rgb(232, 232, 237);
const TEXT: u32 = rgb(24, 24, 27);
const TEXT_MUTED: u32 = rgb(113, 113, 122);
const TEXT_DISABLED: u32 = rgb(170, 170, 178);
const PRIMARY: u32 = rgb(32, 35, 41);
const PRIMARY_PRESSED: u32 = rgb(64, 68, 77);
const ACCENT_SOFT: u32 = rgb(251, 243, 226);
const ACCENT_TEXT: u32 = rgb(138, 98, 30);
const TEXT_OK: u32 = rgb(22, 128, 61);
const TEXT_ERR: u32 = rgb(200, 30, 30);

/// 창 안쪽 크기. 제목 표시줄을 합쳐도 640x480보다 작다.
const CLIENT_W: i32 = 600;
const CLIENT_H: i32 = 432;
const WINDOW_STYLE: u32 = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;

const CARD_ADAPTER: RECT = RECT::ltrb(16, 48, 584, 156);
const CARD_IP: RECT = RECT::ltrb(16, 206, 584, 378);
const SEGMENT: RECT = RECT::ltrb(32, 220, 568, 252);
const BAR_TOP: i32 = 388;
/// 주소 입력 칸: (라벨 오른쪽 끝, 입력칸 x, y)
const FIELD_POS: [(i32, i32, i32); 5] = [
    (116, 124, 266),
    (116, 124, 300),
    (116, 124, 334),
    (372, 380, 266),
    (372, 380, 300),
];
const FIELD_KEYS: [&str; 5] = ["ip", "subnet", "gateway", "dns1", "dns2"];
const FIELD_W: i32 = 150;
/// 프로필은 최대 3개. 하단 바(적용 버튼 왼쪽)에 버튼으로 나온다.
const MAX_PROFILES: usize = 3;
const QUICK_X: i32 = 20;
const QUICK_W: i32 = 140;
const QUICK_GAP: i32 = 8;
const APPLY_RECT: RECT = RECT::new(476, BAR_TOP + 7, 108, 30);

/// 하단 바의 프로필 버튼 위치
const fn quick_rect(index: usize) -> RECT {
    RECT::new(
        QUICK_X + (QUICK_W + QUICK_GAP) * index as i32,
        BAR_TOP + 7,
        QUICK_W,
        30,
    )
}
const FIELD_H: i32 = 26;

// ------------------------------------------------------------
// 컨트롤 ID
// ------------------------------------------------------------
const ID_ADAPTER: i32 = 101;
const ID_REFRESH: i32 = 102;
const ID_SETTINGS: i32 = 103;
const ID_PROFILE: i32 = 104;
const ID_PROF_SAVE: i32 = 105;
const ID_PROF_NEW: i32 = 106;
const ID_PROF_DEL: i32 = 107;
const ID_MODE_DHCP: i32 = 108;
const ID_MODE_STATIC: i32 = 109;
const ID_FIELD0: i32 = 110; // 110..=114
const ID_APPLY: i32 = 115;
const ID_NAME_EDIT: i32 = 116;
const ID_NAME_OK: i32 = 117;
const ID_NAME_CANCEL: i32 = 118;
const ID_QUICK0: i32 = 120; // 120..=122

const MENU_KO: usize = 201;
const MENU_EN: usize = 202;
const MENU_SAVE: usize = 203;
const MENU_FOLDER: usize = 204;

/// 작업 스레드가 끝났음을 알리는 메시지
const WM_APP_WORKER: u32 = WM_APP + 1;

// ------------------------------------------------------------
// GDI 자원
// ------------------------------------------------------------
struct Fonts {
    body: HGDIOBJ,
    body_bold: HGDIOBJ,
    small: HGDIOBJ,
    title: HGDIOBJ,
    big: HGDIOBJ,
    icon: HGDIOBJ,
}

fn font(height: i32, weight: i32, face: &str) -> HGDIOBJ {
    let face = wide(face);
    unsafe {
        CreateFontW(
            -height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            face.as_ptr(),
        )
    }
}

impl Fonts {
    fn new() -> Self {
        const UI: &str = "Malgun Gothic";
        Self {
            body: font(14, FW_NORMAL, UI),
            body_bold: font(14, FW_SEMIBOLD, UI),
            small: font(12, FW_NORMAL, UI),
            title: font(18, FW_BOLD, UI),
            big: font(26, FW_SEMIBOLD, UI),
            // Windows 11 기본 아이콘 글꼴
            icon: font(16, FW_NORMAL, "Segoe Fluent Icons"),
        }
    }
}

fn fill_round(dc: HDC, rect: RECT, radius: i32, fill: u32, border: Option<u32>) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = match border {
            Some(color) => CreatePen(PS_SOLID, 1, color),
            None => CreatePen(PS_SOLID, 1, fill),
        };
        let old_brush = SelectObject(dc, brush);
        let old_pen = SelectObject(dc, pen);
        RoundRect(
            dc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            radius * 2,
            radius * 2,
        );
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

fn fill_rect(dc: HDC, rect: RECT, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
    }
}

fn draw_text(dc: HDC, text: &str, rect: RECT, font: HGDIOBJ, color: u32, format: u32) {
    let text: Vec<u16> = text.encode_utf16().collect();
    let mut rect = rect;
    unsafe {
        let old = SelectObject(dc, font);
        SetTextColor(dc, color);
        DrawTextW(
            dc,
            text.as_ptr(),
            text.len() as i32,
            &mut rect,
            format | DT_NOPREFIX,
        );
        SelectObject(dc, old);
    }
}

fn text_width(dc: HDC, text: &str, font: HGDIOBJ) -> i32 {
    let text: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    unsafe {
        let old = SelectObject(dc, font);
        GetTextExtentPoint32W(dc, text.as_ptr(), text.len() as i32, &mut size);
        SelectObject(dc, old);
    }
    size.cx
}

const LINE: u32 = DT_SINGLELINE | DT_VCENTER;

// ------------------------------------------------------------
// 앱 상태
// ------------------------------------------------------------
enum Work {
    Adapters(Result<Vec<AdapterInfo>, String>),
    Applied(Result<bool, String>),
}

struct Controls {
    adapter: HWND,
    refresh: HWND,
    settings: HWND,
    profile: HWND,
    prof_save: HWND,
    prof_new: HWND,
    prof_del: HWND,
    mode_dhcp: HWND,
    mode_static: HWND,
    fields: [HWND; 5],
    apply: HWND,
    name_edit: HWND,
    name_ok: HWND,
    name_cancel: HWND,
    quick: [HWND; MAX_PROFILES],
}

struct App {
    hwnd: HWND,
    store: Store,
    settings: Settings,
    adapters: Vec<AdapterInfo>,
    adapter_error: Option<String>,
    status: String,
    status_ok: bool,
    /// 적용 중
    busy: bool,
    /// 어댑터 목록을 읽는 중
    loading: bool,
    /// 처음 어댑터 목록을 받은 뒤 기본 프로필을 만들었는지
    defaults_checked: bool,
    /// 새 프로필 이름을 입력받는 중
    naming: bool,
    fonts: Fonts,
    c: Controls,
    tx: Sender<Work>,
    rx: Receiver<Work>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// 앱 상태를 빌려 쓴다. 이미 빌려 쓰는 중(재진입)이면 None.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| {
        let mut guard = cell.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

fn create_child(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    rect: RECT,
    id: i32,
    font: HGDIOBJ,
) -> HWND {
    let class = wide(class);
    let text = wide(text);
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            WS_CHILD | style,
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            parent,
            id as isize,
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

fn show(hwnd: HWND, visible: bool) {
    unsafe {
        ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
    }
}

fn send(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    unsafe { SendMessageW(hwnd, msg, wp, lp) }
}

/// Windows 내장 IP 주소 칸에 값을 넣는다. 올바른 주소가 아니면 비운다.
fn set_field(hwnd: HWND, text: &str) {
    match ipv4_to_u32(text) {
        Some(value) => send(hwnd, IPM_SETADDRESS, 0, value as isize),
        None => send(hwnd, IPM_CLEARADDRESS, 0, 0),
    };
}

/// IP 주소 칸의 값. 모두 비었으면 "", 일부만 채워졌으면 검증에서 걸리도록 그대로 표시한다.
fn get_field(hwnd: HWND) -> String {
    let mut value: u32 = 0;
    let filled = send(hwnd, IPM_GETADDRESS, 0, &mut value as *mut u32 as isize);
    match filled {
        0 => String::new(),
        4 => {
            let [a, b, c, d] = value.to_be_bytes();
            format!("{a}.{b}.{c}.{d}")
        }
        _ => "incomplete".to_string(),
    }
}

/// 어댑터 드롭다운 표시: 별칭 · 실제 장치명 (상태)
fn adapter_label(adapter: &AdapterInfo, lang: &str) -> String {
    let mut label = format!("{} · {}", adapter.name, adapter.description);
    match adapter.status.as_str() {
        "up" => {}
        "disconnected" => label.push_str(&format!(" ({})", tr(lang, "adapter_status_disconnected"))),
        _ => label.push_str(&format!(" ({})", tr(lang, "adapter_status_down"))),
    }
    label
}

impl App {
    fn new(hwnd: HWND) -> Self {
        let fonts = Fonts::new();
        let store = Store::open();
        let mut settings = store.load_settings();
        if !settings.active_profile.is_empty()
            && !settings
                .profiles
                .iter()
                .any(|profile| profile.name == settings.active_profile)
        {
            settings.active_profile.clear();
        }

        let owner = |text: &str, rect: RECT, id: i32, f: HGDIOBJ| {
            create_child(hwnd, "BUTTON", text, WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW, rect, id, f)
        };
        let combo = |rect: RECT, id: i32| {
            create_child(
                hwnd,
                "COMBOBOX",
                "",
                WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST,
                rect,
                id,
                fonts.body,
            )
        };

        let fields = std::array::from_fn(|index| {
            let (_, x, y) = FIELD_POS[index];
            create_child(
                hwnd,
                "SysIPAddress32",
                "",
                WS_TABSTOP,
                RECT::new(x, y, FIELD_W, FIELD_H),
                ID_FIELD0 + index as i32,
                fonts.body,
            )
        });

        let c = Controls {
            settings: owner("\u{E713}", RECT::new(548, 10, 36, 30), ID_SETTINGS, fonts.icon),
            adapter: combo(RECT::new(32, 62, 496, 300), ID_ADAPTER),
            refresh: owner("\u{E72C}", RECT::new(536, 60, 32, 28), ID_REFRESH, fonts.icon),
            profile: combo(RECT::new(80, 167, 246, 300), ID_PROFILE),
            prof_save: owner("", RECT::new(334, 166, 72, 28), ID_PROF_SAVE, fonts.body),
            prof_new: owner("", RECT::new(412, 166, 92, 28), ID_PROF_NEW, fonts.body),
            prof_del: owner("", RECT::new(510, 166, 74, 28), ID_PROF_DEL, fonts.body),
            mode_dhcp: owner(
                "",
                RECT::ltrb(SEGMENT.left + 3, SEGMENT.top + 3, 300, SEGMENT.bottom - 3),
                ID_MODE_DHCP,
                fonts.body,
            ),
            mode_static: owner(
                "",
                RECT::ltrb(300, SEGMENT.top + 3, SEGMENT.right - 3, SEGMENT.bottom - 3),
                ID_MODE_STATIC,
                fonts.body,
            ),
            fields,
            apply: owner("", APPLY_RECT, ID_APPLY, fonts.body_bold),
            name_edit: create_child(
                hwnd,
                "EDIT",
                "",
                WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL,
                RECT::new(80, 168, 246, 25),
                ID_NAME_EDIT,
                fonts.body,
            ),
            name_ok: create_child(
                hwnd,
                "BUTTON",
                "",
                WS_TABSTOP | BS_OWNERDRAW,
                RECT::new(334, 166, 92, 28),
                ID_NAME_OK,
                fonts.body_bold,
            ),
            name_cancel: create_child(
                hwnd,
                "BUTTON",
                "",
                WS_TABSTOP | BS_OWNERDRAW,
                RECT::new(432, 166, 72, 28),
                ID_NAME_CANCEL,
                fonts.body,
            ),
            quick: std::array::from_fn(|index| {
                create_child(
                    hwnd,
                    "BUTTON",
                    "",
                    WS_TABSTOP | BS_OWNERDRAW,
                    quick_rect(index),
                    ID_QUICK0 + index as i32,
                    fonts.body,
                )
            }),
        };
        send(c.adapter, CB_SETMINVISIBLE, 8, 0);
        send(c.profile, CB_SETMINVISIBLE, 12, 0);
        send(c.name_edit, EM_LIMITTEXT, 40, 0);

        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            hwnd,
            store,
            settings,
            adapters: Vec::new(),
            adapter_error: None,
            status: String::new(),
            status_ok: true,
            busy: false,
            loading: false,
            defaults_checked: false,
            naming: false,
            fonts,
            c,
            tx,
            rx,
        };
        app.sync_settings_to_controls();
        app.fill_profile_combo();
        app.update_texts();
        app.start_refresh();
        app
    }

    fn lang(&self) -> String {
        self.settings.language.clone()
    }

    fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    fn redraw_all(&self) {
        unsafe {
            RedrawWindow(
                self.hwnd,
                std::ptr::null(),
                0,
                RDW_INVALIDATE | RDW_ALLCHILDREN,
            );
        }
    }

    fn set_status(&mut self, text: impl Into<String>, ok: bool) {
        self.status = text.into();
        self.status_ok = ok;
        self.invalidate();
    }

    fn fail(&mut self, text: &str) {
        self.set_status(text, false);
    }

    fn save_settings_file(&self) -> bool {
        self.store.save_settings(&self.settings)
    }

    // ---- 작업 스레드 ----
    fn spawn(&self, job: impl FnOnce() -> Work + Send + 'static) {
        let tx = self.tx.clone();
        let hwnd = self.hwnd;
        std::thread::spawn(move || {
            let _ = tx.send(job());
            unsafe {
                PostMessageW(hwnd, WM_APP_WORKER, 0, 0);
            }
        });
    }

    fn start_refresh(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        unsafe {
            EnableWindow(self.c.refresh, 0);
        }
        if self.adapters.is_empty() && self.status.is_empty() {
            self.status = tr(&self.lang(), "status_loading").to_string();
            self.status_ok = true;
        }
        self.invalidate();
        self.spawn(|| Work::Adapters(list_adapters()));
    }

    fn drain_worker(&mut self) {
        while let Ok(work) = self.rx.try_recv() {
            match work {
                Work::Adapters(result) => self.on_adapters(result),
                Work::Applied(result) => self.on_applied(result),
            }
        }
    }

    fn on_adapters(&mut self, result: Result<Vec<AdapterInfo>, String>) {
        self.loading = false;
        unsafe {
            EnableWindow(self.c.refresh, 1);
        }
        let lang = self.lang();
        if self.status == tr(&lang, "status_loading") {
            self.status.clear();
        }
        match result {
            Ok(list) => {
                self.adapters = list;
                self.adapter_error = None;
                self.resolve_selection();
            }
            Err(err) => self.adapter_error = Some(err),
        }
        if !self.defaults_checked {
            self.defaults_checked = true;
            self.ensure_default_profiles();
        }
        self.fill_adapter_combo();
        self.invalidate();
    }

    fn on_applied(&mut self, result: Result<bool, String>) {
        self.busy = false;
        unsafe {
            EnableWindow(self.c.apply, 1);
        }
        let lang = self.lang();
        match result {
            Ok(use_static) => {
                let key = if use_static {
                    "status_static_done"
                } else {
                    "status_dhcp_done"
                };
                self.set_status(tr(&lang, key), true);
                self.save_settings_file();
                self.start_refresh();
            }
            Err(err) => {
                let message = tr(&lang, "status_error").replace("{}", &err);
                self.fail(&message);
            }
        }
    }

    // ---- 어댑터 ----
    fn selected_adapter(&self) -> Option<&AdapterInfo> {
        self.adapters
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(&self.settings.adapter))
    }

    /// 저장된 어댑터를 유지하고, 목록에 없으면 첫 Wi-Fi 어댑터를 고른다.
    fn resolve_selection(&mut self) {
        let saved = self.settings.adapter.trim().to_string();
        if let Some(found) = self
            .adapters
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(&saved))
        {
            self.settings.adapter = found.name.clone();
            return;
        }
        if let Some(adapter) = self.adapters.first() {
            self.settings.adapter = adapter.name.clone();
            self.save_settings_file();
        }
    }

    fn fill_adapter_combo(&self) {
        let lang = self.lang();
        send(self.c.adapter, CB_RESETCONTENT, 0, 0);
        let cue = wide(tr(&lang, "adapter_none"));
        send(self.c.adapter, CB_SETCUEBANNER, 0, cue.as_ptr() as isize);
        let mut selected = -1isize;
        for (index, adapter) in self.adapters.iter().enumerate() {
            let label = wide(&adapter_label(adapter, &lang));
            send(self.c.adapter, CB_ADDSTRING, 0, label.as_ptr() as isize);
            if adapter.name.eq_ignore_ascii_case(&self.settings.adapter) {
                selected = index as isize;
            }
        }
        send(self.c.adapter, CB_SETCURSEL, selected as usize, 0);
    }

    // ---- 프로필 ----
    fn ensure_default_profiles(&mut self) {
        if !self.settings.profiles.is_empty() {
            return;
        }
        let adapter = self.selected_adapter().or(self.adapters.first()).cloned();
        let adapter_name = adapter.as_ref().map(|a| a.name.clone()).unwrap_or_default();

        // 외부, 테더링 - DHCP 프로필
        self.settings.profiles.push(Profile {
            name: "외부, 테더링".to_string(),
            adapter: adapter_name,
            use_static: false,
            ..Default::default()
        });

        // 사내 IP - 현재 무선랜에 설정되어 있는 IP 프로필
        if let Some(adapter) = adapter {
            if let Some(ip) = adapter.ip.clone() {
                self.settings.profiles.push(Profile {
                    name: "사내 IP".to_string(),
                    adapter: adapter.name.clone(),
                    use_static: true,
                    ip,
                    subnet: normalize_subnet(adapter.mask.as_deref().unwrap_or("255.255.255.0")),
                    gateway: adapter.gateway.clone().unwrap_or_default(),
                    dns1: adapter.dns.first().cloned().unwrap_or_default(),
                    dns2: adapter.dns.get(1).cloned().unwrap_or_default(),
                });
            }
        }
        self.settings.sort_profiles();
        self.save_settings_file();
        self.fill_profile_combo();
    }

    fn fill_profile_combo(&self) {
        let lang = self.lang();
        send(self.c.profile, CB_RESETCONTENT, 0, 0);
        let cue = wide(tr(&lang, "profile_pick"));
        send(self.c.profile, CB_SETCUEBANNER, 0, cue.as_ptr() as isize);
        let mut selected = -1isize;
        for (index, profile) in self.settings.profiles.iter().enumerate() {
            let name = wide(&profile.name);
            send(self.c.profile, CB_ADDSTRING, 0, name.as_ptr() as isize);
            if profile.name == self.settings.active_profile {
                selected = index as isize;
            }
        }
        send(self.c.profile, CB_SETCURSEL, selected as usize, 0);
        unsafe {
            EnableWindow(
                self.c.prof_del,
                (!self.settings.active_profile.is_empty()) as i32,
            );
            EnableWindow(
                self.c.prof_new,
                (self.settings.profiles.len() < MAX_PROFILES) as i32,
            );
        }
        self.update_quick_buttons();
    }

    /// 하단 바의 프로필 버튼(최대 3개)을 프로필 목록에 맞춘다.
    fn update_quick_buttons(&self) {
        for (index, hwnd) in self.c.quick.iter().enumerate() {
            match self.settings.profiles.get(index) {
                Some(profile) => {
                    set_text(*hwnd, &profile.name);
                    show(*hwnd, true);
                    unsafe {
                        InvalidateRect(*hwnd, std::ptr::null(), 0);
                    }
                }
                None => show(*hwnd, false),
            }
        }
    }

    fn select_profile(&mut self, index: usize) {
        let Some(profile) = self.settings.profiles.get(index).cloned() else {
            return;
        };
        profile.apply_to(&mut self.settings);
        self.settings.active_profile = profile.name.clone();
        self.resolve_selection();
        self.save_settings_file();
        self.sync_settings_to_controls();
        self.fill_adapter_combo();
        self.fill_profile_combo();
        let lang = self.lang();
        self.set_status(
            tr(&lang, "status_profile_loaded").replace("{}", &profile.name),
            true,
        );
    }

    /// 지금 화면의 값으로 프로필을 추가하거나 덮어쓴다.
    fn persist_profile(&mut self, name: &str) -> bool {
        let lang = self.lang();
        let name = match validate_profile_name(name) {
            Ok(name) => name,
            Err(key) => {
                self.fail(tr(&lang, key));
                return false;
            }
        };
        let exists = self
            .settings
            .profiles
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(&name));
        if !exists && self.settings.profiles.len() >= MAX_PROFILES {
            self.fail(tr(&lang, "profile_limit"));
            return false;
        }
        self.sync_controls_to_settings();
        let profile = Profile::from_settings(&name, &self.settings);
        match self
            .settings
            .profiles
            .iter()
            .position(|existing| existing.name.eq_ignore_ascii_case(&profile.name))
        {
            Some(index) => self.settings.profiles[index] = profile.clone(),
            None => self.settings.profiles.push(profile.clone()),
        }
        self.settings.sort_profiles();
        self.settings.active_profile = profile.name.clone();
        self.fill_profile_combo();
        if !self.save_settings_file() {
            let message =
                tr(&lang, "status_error").replace("{}", tr(&lang, "status_save_failed"));
            self.fail(&message);
            return false;
        }
        self.set_status(
            tr(&lang, "status_profile_saved").replace("{}", &profile.name),
            true,
        );
        true
    }

    fn save_current_profile(&mut self) {
        if self.settings.active_profile.is_empty() {
            self.begin_naming();
        } else {
            let name = self.settings.active_profile.clone();
            self.persist_profile(&name);
        }
    }

    fn delete_profile(&mut self, name: &str) {
        let before = self.settings.profiles.len();
        self.settings
            .profiles
            .retain(|profile| !profile.name.eq_ignore_ascii_case(name));
        if self.settings.profiles.len() == before {
            return;
        }
        if self.settings.active_profile.eq_ignore_ascii_case(name) {
            self.settings.active_profile.clear();
        }
        self.save_settings_file();
        self.fill_profile_combo();
        let lang = self.lang();
        self.set_status(tr(&lang, "status_profile_deleted").replace("{}", name), true);
    }

    fn suggested_profile_name(&self) -> String {
        let lang = self.lang();
        let mut index = self.settings.profiles.len() + 1;
        loop {
            let name = tr(&lang, "profile_default_name").replace("{}", &index.to_string());
            if !self.settings.profiles.iter().any(|p| p.name == name) {
                return name;
            }
            index += 1;
        }
    }

    /// 프로필 줄을 이름 입력 줄로 바꾼다.
    fn begin_naming(&mut self) {
        if self.settings.profiles.len() >= MAX_PROFILES {
            let lang = self.lang();
            self.fail(tr(&lang, "profile_limit"));
            return;
        }
        self.naming = true;
        let lang = self.lang();
        set_text(self.c.name_edit, "");
        let cue = wide(&format!(
            "{} ({})",
            tr(&lang, "profile_name"),
            self.suggested_profile_name()
        ));
        send(self.c.name_edit, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
        self.update_naming_visibility();
        unsafe {
            SetFocus(self.c.name_edit);
        }
    }

    fn end_naming(&mut self) {
        self.naming = false;
        self.update_naming_visibility();
        unsafe {
            SetFocus(self.c.profile);
        }
    }

    fn update_naming_visibility(&self) {
        for hwnd in [self.c.profile, self.c.prof_save, self.c.prof_new, self.c.prof_del] {
            show(hwnd, !self.naming);
        }
        for hwnd in [self.c.name_edit, self.c.name_ok, self.c.name_cancel] {
            show(hwnd, self.naming);
        }
        self.invalidate();
    }

    fn create_profile_from_edit(&mut self) {
        let mut name = window_text(self.c.name_edit);
        if name.trim().is_empty() {
            name = self.suggested_profile_name();
        }
        let lang = self.lang();
        if let Err(key) = validate_profile_name(&name) {
            self.fail(tr(&lang, key));
            unsafe {
                SetFocus(self.c.name_edit);
            }
            return;
        }
        if self
            .settings
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(name.trim()))
        {
            self.fail(tr(&lang, "status_profile_exists"));
            unsafe {
                SetFocus(self.c.name_edit);
            }
            return;
        }
        if self.persist_profile(&name) {
            self.end_naming();
        }
    }

    // ---- 입력칸 ----
    fn sync_settings_to_controls(&mut self) {
        let values = [
            self.settings.ip.clone(),
            self.settings.subnet.clone(),
            self.settings.gateway.clone(),
            self.settings.dns1.clone(),
            self.settings.dns2.clone(),
        ];
        for (hwnd, value) in self.c.fields.iter().zip(values) {
            set_field(*hwnd, &value);
        }
        self.update_mode_visibility();
    }

    fn sync_controls_to_settings(&mut self) {
        let [ip, subnet, gateway, dns1, dns2] = self.c.fields.map(get_field);
        self.settings.ip = ip;
        self.settings.subnet = subnet;
        self.settings.gateway = gateway;
        self.settings.dns1 = dns1;
        self.settings.dns2 = dns2;
    }

    fn update_mode_visibility(&self) {
        for hwnd in self.c.fields {
            show(hwnd, self.settings.use_static);
        }
        unsafe {
            InvalidateRect(self.c.mode_dhcp, std::ptr::null(), 0);
            InvalidateRect(self.c.mode_static, std::ptr::null(), 0);
        }
        self.invalidate();
    }

    fn set_mode(&mut self, use_static: bool) {
        if self.settings.use_static != use_static {
            self.settings.use_static = use_static;
            self.update_mode_visibility();
        }
    }

    // ---- 적용 ----
    fn apply(&mut self) {
        if self.busy {
            return;
        }
        let lang = self.lang();
        if self.settings.adapter.trim().is_empty() || self.adapters.is_empty() {
            self.fail(tr(&lang, "status_select_adapter"));
            return;
        }
        if self.selected_adapter().is_none() {
            self.fail(tr(&lang, "status_adapter_missing"));
            return;
        }
        let adapter = self.settings.adapter.clone();
        self.sync_controls_to_settings();
        let use_static = self.settings.use_static;
        let (ip, gateway, dns1, dns2) = (
            self.settings.ip.clone(),
            self.settings.gateway.clone(),
            self.settings.dns1.clone(),
            self.settings.dns2.clone(),
        );
        let mut prefix = String::new();
        if use_static {
            match validate_static(&ip, &self.settings.subnet, &gateway, &dns1, &dns2) {
                Ok(bits) => {
                    prefix = bits.to_string();
                    self.settings.subnet = prefix_to_mask(bits);
                }
                Err(key) => {
                    self.fail(tr(&lang, key));
                    return;
                }
            }
        }

        self.busy = true;
        unsafe {
            EnableWindow(self.c.apply, 0);
        }
        self.set_status(tr(&lang, "status_applying"), true);
        self.spawn(move || {
            Work::Applied(
                apply_settings(&adapter, use_static, &ip, &prefix, &gateway, &dns1, &dns2)
                    .map(|_| use_static),
            )
        });
    }

    // ---- 메뉴 ----
    fn on_menu(&mut self, id: usize) {
        match id {
            MENU_KO | MENU_EN => {
                self.settings.language = if id == MENU_EN { "en" } else { "ko" }.to_string();
                self.save_settings_file();
                self.status.clear();
                self.update_texts();
                self.fill_adapter_combo();
                self.fill_profile_combo();
                self.redraw_all();
            }
            MENU_SAVE => {
                self.sync_controls_to_settings();
                self.settings.subnet = match parse_subnet(&self.settings.subnet) {
                    Some(bits) => prefix_to_mask(bits),
                    None => self.settings.subnet.trim().to_string(),
                };
                self.save_settings_file();
                let lang = self.lang();
                self.set_status(tr(&lang, "status_saved"), true);
            }
            MENU_FOLDER => {
                let _ = Command::new("explorer").arg(self.store.root()).spawn();
            }
            _ => {}
        }
    }

    /// 접근성(화면 읽기)용 버튼 이름. 실제 글자는 직접 그린다.
    fn update_texts(&self) {
        let lang = self.lang();
        for (hwnd, key) in [
            (self.c.refresh, "refresh"),
            (self.c.prof_save, "profile_save"),
            (self.c.prof_new, "profile_new"),
            (self.c.prof_del, "profile_delete"),
            (self.c.mode_dhcp, "mode_dhcp"),
            (self.c.mode_static, "mode_static"),
            (self.c.apply, "apply"),
            (self.c.name_ok, "profile_create"),
            (self.c.name_cancel, "cancel"),
        ] {
            set_text(hwnd, tr(&lang, key));
        }
    }

    fn command(&mut self, id: i32, code: u32) {
        let clicked = code == BN_CLICKED || code == BN_DOUBLECLICKED;
        match id {
            ID_ADAPTER if code == CBN_SELCHANGE => {
                let index = send(self.c.adapter, CB_GETCURSEL, 0, 0);
                if let Some(adapter) = usize::try_from(index).ok().and_then(|i| self.adapters.get(i))
                {
                    self.settings.adapter = adapter.name.clone();
                    self.save_settings_file();
                    self.invalidate();
                }
            }
            ID_PROFILE if code == CBN_SELCHANGE => {
                let index = send(self.c.profile, CB_GETCURSEL, 0, 0);
                if let Ok(index) = usize::try_from(index) {
                    self.select_profile(index);
                }
            }
            ID_REFRESH if clicked => self.start_refresh(),
            ID_PROF_SAVE if clicked => self.save_current_profile(),
            ID_PROF_NEW if clicked => self.begin_naming(),
            ID_MODE_DHCP if clicked => self.set_mode(false),
            ID_MODE_STATIC if clicked => self.set_mode(true),
            ID_APPLY if clicked => self.apply(),
            id if clicked && (ID_QUICK0..ID_QUICK0 + MAX_PROFILES as i32).contains(&id) => {
                self.select_profile((id - ID_QUICK0) as usize);
            }
            ID_NAME_OK if clicked => self.create_profile_from_edit(),
            ID_NAME_CANCEL if clicked => self.end_naming(),
            // 이름 입력칸에서 Enter / Esc
            IDOK if self.naming => self.create_profile_from_edit(),
            IDCANCEL if self.naming => self.end_naming(),
            _ => {}
        }
    }

    // ---- 그리기 ----
    fn paint(&self, target: HDC) {
        unsafe {
            let dc = CreateCompatibleDC(target);
            let bitmap = CreateCompatibleBitmap(target, CLIENT_W, CLIENT_H);
            let old_bitmap = SelectObject(dc, bitmap);
            SetBkMode(dc, TRANSPARENT);
            self.paint_scene(dc);
            BitBlt(target, 0, 0, CLIENT_W, CLIENT_H, dc, 0, 0, SRCCOPY);
            SelectObject(dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(dc);
        }
    }

    fn paint_scene(&self, dc: HDC) {
        let lang = self.lang();
        let f = &self.fonts;
        fill_rect(dc, RECT::new(0, 0, CLIENT_W, CLIENT_H), PANEL_BG);

        // 제목 + 오른쪽에 상태 메시지
        draw_text(dc, tr(&lang, "title"), RECT::ltrb(20, 8, 160, 42), f.title, TEXT, LINE);
        if !self.status.is_empty() {
            let color = if self.status_ok { TEXT_OK } else { TEXT_ERR };
            let text_color = if self.status_ok { TEXT_MUTED } else { TEXT_ERR };
            let right = 538;
            let width = text_width(dc, &self.status, f.small).min(right - 168);
            draw_text(
                dc,
                &self.status,
                RECT::ltrb(right - width, 8, right, 42),
                f.small,
                text_color,
                LINE | DT_RIGHT | DT_END_ELLIPSIS,
            );
            fill_round(dc, RECT::new(right - width - 14, 21, 8, 8), 4, color, None);
        }

        // 현재 연결 카드
        fill_round(dc, CARD_ADAPTER, 10, CARD_BG, Some(CARD_STROKE));
        let adapter = self.selected_adapter();
        match adapter.and_then(|a| a.ip.as_deref().map(|ip| (a, ip))) {
            Some((adapter, ip)) => {
                let ip_rect = RECT::ltrb(32, 94, 400, 128);
                draw_text(dc, ip, ip_rect, f.big, TEXT, LINE);
                let mut x = 32 + text_width(dc, ip, f.big) + 12;
                let (mode, fill, color) = if adapter.dhcp {
                    (tr(&lang, "current_dhcp"), SEGMENT_BG, TEXT_MUTED)
                } else {
                    (tr(&lang, "current_static"), ACCENT_SOFT, ACCENT_TEXT)
                };
                x = pill(dc, f, x, 102, mode, fill, color);
                if adapter.status != "up" {
                    let key = if adapter.status == "disconnected" {
                        "adapter_status_disconnected"
                    } else {
                        "adapter_status_down"
                    };
                    pill(dc, f, x + 6, 102, tr(&lang, key), rgb(253, 236, 236), TEXT_ERR);
                }
                let dns = if adapter.dns.is_empty() {
                    "-".to_string()
                } else {
                    adapter.dns.join(", ")
                };
                let mut x = 32;
                for (key, value) in [
                    ("subnet", adapter.mask.clone().unwrap_or_else(|| "-".into())),
                    ("gateway", adapter.gateway.clone().unwrap_or_else(|| "-".into())),
                    ("dns", dns),
                ] {
                    let label = tr(&lang, key);
                    draw_text(dc, label, RECT::ltrb(x, 128, 584, 148), f.small, TEXT_MUTED, LINE);
                    x += text_width(dc, label, f.small) + 6;
                    draw_text(dc, &value, RECT::ltrb(x, 128, 568, 148), f.small, TEXT, LINE | DT_END_ELLIPSIS);
                    x += text_width(dc, &value, f.small) + 18;
                }
            }
            None => {
                let text = if self.loading && self.adapters.is_empty() {
                    tr(&lang, "status_loading")
                } else {
                    tr(&lang, "current_none")
                };
                draw_text(dc, text, RECT::ltrb(32, 94, 568, 128), f.body_bold, TEXT_MUTED, LINE);
            }
        }
        if let Some(error) = &self.adapter_error {
            let text = format!(
                "{} {}",
                tr(&lang, "status_adapter_list_failed"),
                truncate_chars(error, 80)
            );
            draw_text(dc, &text, RECT::ltrb(32, 128, 568, 148), f.small, TEXT_ERR, LINE | DT_END_ELLIPSIS);
        }

        // 프로필 줄
        draw_text(dc, tr(&lang, "profile"), RECT::ltrb(20, 166, 80, 194), f.body_bold, TEXT_MUTED, LINE);

        // IP 설정 카드
        fill_round(dc, CARD_IP, 10, CARD_BG, Some(CARD_STROKE));
        fill_round(dc, SEGMENT, 8, SEGMENT_BG, None);
        if self.settings.use_static {
            for (index, key) in FIELD_KEYS.iter().enumerate() {
                let (label_right, _, y) = FIELD_POS[index];
                draw_text(
                    dc,
                    tr(&lang, key),
                    RECT::ltrb(label_right - 100, y, label_right, y + FIELD_H),
                    f.body,
                    TEXT_MUTED,
                    LINE | DT_RIGHT,
                );
            }
        } else {
            draw_text(
                dc,
                tr(&lang, "dhcp_note"),
                RECT::ltrb(32, 266, 568, 300),
                f.body,
                TEXT_MUTED,
                LINE,
            );
        }

        // 하단 바
        fill_rect(dc, RECT::ltrb(0, BAR_TOP, CLIENT_W, CLIENT_H), CARD_BG);
        fill_rect(dc, RECT::ltrb(0, BAR_TOP, CLIENT_W, BAR_TOP + 1), CARD_STROKE);
    }

    fn draw_item(&self, item: &DRAWITEMSTRUCT) {
        let dc = item.hDC;
        let rect = item.rcItem;
        let id = item.CtlID as i32;
        let pressed = item.itemState & ODS_SELECTED != 0;
        let disabled = item.itemState & ODS_DISABLED != 0;
        let focused = item.itemState & ODS_FOCUS != 0;
        let f = &self.fonts;
        let label = window_text(item.hwndItem);
        unsafe {
            SetBkMode(dc, TRANSPARENT);
        }

        match id {
            ID_MODE_DHCP | ID_MODE_STATIC => {
                fill_rect(dc, rect, SEGMENT_BG);
                let selected = (id == ID_MODE_STATIC) == self.settings.use_static;
                if selected {
                    fill_round(dc, rect, 7, CARD_BG, Some(CARD_STROKE));
                } else if pressed {
                    fill_round(dc, rect, 7, PRESSED_BG, None);
                }
                let (font, color) = if selected {
                    (f.body_bold, TEXT)
                } else {
                    (f.body, TEXT_MUTED)
                };
                draw_text(dc, &label, rect, font, color, LINE | DT_CENTER);
            }
            ID_APPLY | ID_NAME_OK => {
                let under = if id == ID_APPLY { CARD_BG } else { PANEL_BG };
                fill_rect(dc, rect, under);
                let fill = if disabled {
                    SEGMENT_BG
                } else if pressed {
                    PRIMARY_PRESSED
                } else {
                    PRIMARY
                };
                fill_round(dc, rect, 8, fill, None);
                let color = if disabled { TEXT_DISABLED } else { CARD_BG };
                draw_text(dc, &label, rect, f.body_bold, color, LINE | DT_CENTER);
            }
            id if (ID_QUICK0..ID_QUICK0 + MAX_PROFILES as i32).contains(&id) => {
                fill_rect(dc, rect, CARD_BG);
                let index = (id - ID_QUICK0) as usize;
                let selected = self
                    .settings
                    .profiles
                    .get(index)
                    .is_some_and(|p| p.name == self.settings.active_profile);
                let (fill, border, color, font) = if selected {
                    (ACCENT_SOFT, ACCENT_TEXT, ACCENT_TEXT, f.body_bold)
                } else if pressed {
                    (PRESSED_BG, BUTTON_STROKE, TEXT, f.body)
                } else {
                    (CARD_BG, if focused { TEXT_MUTED } else { BUTTON_STROKE }, TEXT, f.body)
                };
                fill_round(dc, rect, 8, fill, Some(border));
                let mut inner = rect;
                inner.left += 8;
                inner.right -= 8;
                draw_text(dc, &label, inner, font, color, LINE | DT_CENTER | DT_END_ELLIPSIS);
            }
            _ => {
                // 흰 바탕 보조 버튼 / 아이콘 버튼
                let under = if id == ID_REFRESH { CARD_BG } else { PANEL_BG };
                fill_rect(dc, rect, under);
                let fill = if pressed { PRESSED_BG } else { CARD_BG };
                let border = if focused { TEXT_MUTED } else { BUTTON_STROKE };
                fill_round(dc, rect, 8, fill, Some(border));
                let color = if disabled { TEXT_DISABLED } else { TEXT };
                let font = if matches!(id, ID_REFRESH | ID_SETTINGS) {
                    f.icon
                } else {
                    f.body
                };
                let text = match id {
                    ID_REFRESH => "\u{E72C}".to_string(),
                    ID_SETTINGS => "\u{E713}".to_string(),
                    _ => label,
                };
                draw_text(dc, &text, rect, font, color, LINE | DT_CENTER);
            }
        }
    }

    /// 창 안쪽 크기를 고정 크기로 맞춘다. (배율이 바뀌어도 안쪽은 같은 픽셀 크기)
    fn fit_window(&self, origin: Option<(i32, i32)>, dpi: u32) {
        let mut rect = RECT::new(0, 0, CLIENT_W, CLIENT_H);
        unsafe {
            AdjustWindowRectExForDpi(&mut rect, WINDOW_STYLE, 0, 0, dpi.max(96));
            let (flags, x, y) = match origin {
                Some((x, y)) => (SWP_NOZORDER | SWP_NOACTIVATE, x, y),
                None => (SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE, 0, 0),
            };
            SetWindowPos(self.hwnd, 0, x, y, rect.width(), rect.height(), flags);
        }
    }
}

/// 둥근 배지. 다음 그릴 x 위치를 돌려준다.
fn pill(dc: HDC, f: &Fonts, x: i32, y: i32, text: &str, fill: u32, color: u32) -> i32 {
    let width = text_width(dc, text, f.small) + 16;
    let rect = RECT::new(x, y, width, 20);
    fill_round(dc, rect, 10, fill, None);
    draw_text(dc, text, rect, f.small, color, LINE | DT_CENTER);
    rect.right
}

// ------------------------------------------------------------
// 창 프로시저
// ------------------------------------------------------------
fn show_settings_menu(hwnd: HWND) {
    let Some((lang, button)) = with_app(|app| (app.lang(), app.c.settings)) else {
        return;
    };
    unsafe {
        let menu = CreatePopupMenu();
        let add = |flags: u32, id: usize, key: &str| {
            let text = wide(tr(&lang, key));
            AppendMenuW(menu, flags, id, text.as_ptr());
        };
        add(MF_STRING | if lang == "en" { 0 } else { MF_CHECKED }, MENU_KO, "lang_ko");
        add(MF_STRING | if lang == "en" { MF_CHECKED } else { 0 }, MENU_EN, "lang_en");
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        add(MF_STRING, MENU_SAVE, "menu_save");
        add(MF_STRING, MENU_FOLDER, "menu_open_folder");
        let mut rect = RECT::default();
        GetWindowRect(button, &mut rect);
        let picked = TrackPopupMenu(menu, TPM_RETURNCMD, rect.left, rect.bottom + 2, 0, hwnd, std::ptr::null());
        DestroyMenu(menu);
        if picked > 0 {
            with_app(|app| app.on_menu(picked as usize));
        }
    }
}

fn confirm_delete(hwnd: HWND) {
    let Some((lang, name)) = with_app(|app| (app.lang(), app.settings.active_profile.clone()))
    else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let text = wide(&tr(&lang, "profile_delete_confirm").replace("{}", &name));
    let caption = wide(tr(&lang, "title"));
    let answer =
        unsafe { MessageBoxW(hwnd, text.as_ptr(), caption.as_ptr(), MB_YESNO | MB_ICONQUESTION) };
    if answer == IDYES {
        with_app(|app| app.delete_profile(&name));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            with_app(|app| app.paint(dc));
            EndPaint(hwnd, &ps);
            return 0;
        }
        // 테마가 적용된 컨트롤이 뒷배경을 요청할 때
        WM_PRINTCLIENT => {
            with_app(|app| app.paint(wp as HDC));
            return 0;
        }
        WM_DRAWITEM => {
            let item = &*(lp as *const DRAWITEMSTRUCT);
            if with_app(|app| app.draw_item(item)).is_none() {
                InvalidateRect(item.hwndItem, std::ptr::null(), 0);
            }
            return 1;
        }
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as i32;
            let code = ((wp >> 16) & 0xFFFF) as u32;
            let clicked = code == BN_CLICKED || code == BN_DOUBLECLICKED;
            match id {
                // 메뉴·확인 창은 앱 상태를 빌리지 않은 채로 띄운다.
                ID_SETTINGS if clicked => show_settings_menu(hwnd),
                ID_PROF_DEL if clicked => confirm_delete(hwnd),
                _ => {
                    with_app(|app| app.command(id, code));
                }
            }
            return 0;
        }
        WM_APP_WORKER => {
            with_app(App::drain_worker);
            return 0;
        }
        // 배율이 다른 모니터로 옮겨도 안쪽 크기는 그대로 둔다.
        WM_DPICHANGED => {
            let suggested = &*(lp as *const RECT);
            let dpi = (wp & 0xFFFF) as u32;
            with_app(|app| app.fit_window(Some((suggested.left, suggested.top)), dpi));
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

fn main() {
    unsafe {
        let init = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_INTERNET_CLASSES,
        };
        InitCommonControlsEx(&init);

        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide("IPChangerWindow");
        // build.rs가 exe에 넣은 아이콘 (리소스 ID 1)
        let icon = |size: i32| {
            LoadImageW(instance, 1 as *const u16, IMAGE_ICON, size, size, LR_SHARED)
        };
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: icon(32),
            hCursor: LoadCursorW(0, IDC_ARROW as *const u16),
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: icon(16),
        };
        RegisterClassExW(&class);

        let title = wide("IP Changer");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WINDOW_STYLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CLIENT_W,
            CLIENT_H,
            0,
            0,
            instance,
            std::ptr::null_mut(),
        );
        if hwnd == 0 {
            return;
        }

        let app = App::new(hwnd);
        app.fit_window(None, GetDpiForWindow(hwnd));
        APP.with(|cell| *cell.borrow_mut() = Some(app));

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            // Tab 이동, Enter/Esc 처리
            if IsDialogMessageW(hwnd, &mut msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_smaller_than_640x480() {
        // 제목 표시줄(약 31px)과 테두리를 더해도 640x480보다 작아야 한다.
        assert!(CLIENT_W + 16 < 640);
        assert!(CLIENT_H + 40 < 480);
    }

    #[test]
    fn layout_fits_inside_window() {
        for rect in [CARD_ADAPTER, CARD_IP, SEGMENT] {
            assert!(rect.left >= 0 && rect.right <= CLIENT_W, "{rect:?}");
            assert!(rect.top >= 0 && rect.bottom <= BAR_TOP, "{rect:?}");
        }
        for (label_right, x, y) in FIELD_POS {
            assert!(label_right < x);
            assert!(x + FIELD_W <= CARD_IP.right - 16);
            assert!(y >= SEGMENT.bottom && y + FIELD_H <= CARD_IP.bottom - 8);
        }
    }

    #[test]
    fn profile_buttons_fit_left_of_apply() {
        for index in 0..MAX_PROFILES {
            let rect = quick_rect(index);
            assert!(rect.left >= 16 && rect.bottom <= CLIENT_H);
            assert!(rect.top >= BAR_TOP);
            if index + 1 < MAX_PROFILES {
                assert!(rect.right < quick_rect(index + 1).left);
            }
        }
        assert!(quick_rect(MAX_PROFILES - 1).right + 8 <= APPLY_RECT.left);
    }

    #[test]
    fn every_ui_text_is_translated() {
        let keys = [
            "title", "refresh", "mode_static", "mode_dhcp", "dhcp_note", "ip", "subnet",
            "gateway", "dns1", "dns2", "dns", "apply", "status_loading", "profile",
            "profile_pick", "profile_save", "profile_new", "profile_delete", "profile_create",
            "cancel", "menu_save", "menu_open_folder", "adapter_none", "profile_limit",
        ];
        for key in keys {
            assert_ne!(tr("ko", key), key, "missing ko text: {key}");
            assert_ne!(tr("en", key), key, "missing en text: {key}");
        }
    }

    #[test]
    fn adapter_label_shows_status() {
        let mut adapter = AdapterInfo {
            name: "Wi-Fi".into(),
            description: "Intel AX211".into(),
            status: "up".into(),
            ..Default::default()
        };
        assert_eq!(adapter_label(&adapter, "ko"), "Wi-Fi · Intel AX211");
        adapter.status = "disconnected".into();
        assert_eq!(adapter_label(&adapter, "ko"), "Wi-Fi · Intel AX211 (연결 안 됨)");
    }
}
