//! Windows 기본 DLL(user32 / gdi32 / kernel32 / comctl32)만 직접 선언해서 쓴다.
#![allow(non_snake_case, clippy::upper_case_acronyms, dead_code)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HDC = isize;
pub type HGDIOBJ = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl RECT {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }
    pub const fn ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: isize,
    pub hIcon: isize,
    pub hCursor: isize,
    pub hbrBackground: isize,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: isize,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: i32,
    pub rcPaint: RECT,
    pub fRestore: i32,
    pub fIncUpdate: i32,
    pub rgbReserved: [u8; 32],
}

#[repr(C)]
pub struct DRAWITEMSTRUCT {
    pub CtlType: u32,
    pub CtlID: u32,
    pub itemID: u32,
    pub itemAction: u32,
    pub itemState: u32,
    pub hwndItem: HWND,
    pub hDC: HDC,
    pub rcItem: RECT,
    pub itemData: usize,
}

#[repr(C)]
pub struct INITCOMMONCONTROLSEX {
    pub dwSize: u32,
    pub dwICC: u32,
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(wc: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: isize,
        instance: isize,
        param: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(msg: *const MSG) -> i32;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn IsDialogMessageW(hwnd: HWND, msg: *mut MSG) -> i32;
    pub fn PostQuitMessage(code: i32);
    pub fn PostMessageW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> i32;
    pub fn SendMessageW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    pub fn UpdateWindow(hwnd: HWND) -> i32;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    pub fn RedrawWindow(hwnd: HWND, rect: *const RECT, region: isize, flags: u32) -> i32;
    pub fn BeginPaint(hwnd: HWND, ps: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, ps: *const PAINTSTRUCT) -> i32;
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn FillRect(dc: HDC, rect: *const RECT, brush: isize) -> i32;
    pub fn DrawTextW(dc: HDC, text: *const u16, len: i32, rect: *mut RECT, format: u32) -> i32;
    pub fn SetWindowTextW(hwnd: HWND, text: *const u16) -> i32;
    pub fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    pub fn SetWindowPos(
        hwnd: HWND,
        after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    pub fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
    pub fn IsWindowVisible(hwnd: HWND) -> i32;
    pub fn LoadCursorW(instance: isize, name: *const u16) -> isize;
    pub fn LoadImageW(
        instance: isize,
        name: *const u16,
        kind: u32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> isize;
    pub fn CreatePopupMenu() -> isize;
    pub fn AppendMenuW(menu: isize, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn TrackPopupMenu(
        menu: isize,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: HWND,
        rect: *const RECT,
    ) -> i32;
    pub fn DestroyMenu(menu: isize) -> i32;
    pub fn MessageBoxW(hwnd: HWND, text: *const u16, caption: *const u16, kind: u32) -> i32;
    pub fn SetFocus(hwnd: HWND) -> HWND;
    pub fn GetDpiForWindow(hwnd: HWND) -> u32;
    pub fn AdjustWindowRectExForDpi(
        rect: *mut RECT,
        style: u32,
        menu: i32,
        ex_style: u32,
        dpi: u32,
    ) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> HGDIOBJ;
    pub fn SelectObject(dc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(obj: HGDIOBJ) -> i32;
    pub fn CreateSolidBrush(color: u32) -> HGDIOBJ;
    pub fn CreatePen(style: i32, width: i32, color: u32) -> HGDIOBJ;
    pub fn RoundRect(dc: HDC, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> i32;
    pub fn Ellipse(dc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
    pub fn SetBkMode(dc: HDC, mode: i32) -> i32;
    pub fn SetTextColor(dc: HDC, color: u32) -> u32;
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(dc: HDC, w: i32, h: i32) -> HGDIOBJ;
    pub fn BitBlt(
        dst: HDC,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: HDC,
        sx: i32,
        sy: i32,
        rop: u32,
    ) -> i32;
    pub fn DeleteDC(dc: HDC) -> i32;
    pub fn GetStockObject(index: i32) -> HGDIOBJ;
    pub fn GetTextExtentPoint32W(dc: HDC, text: *const u16, len: i32, size: *mut SIZE) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> isize;
}

#[link(name = "comctl32")]
extern "system" {
    pub fn InitCommonControlsEx(init: *const INITCOMMONCONTROLSEX) -> i32;
}

// ---- 메시지 ----
pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_SETFONT: u32 = 0x0030;
pub const WM_DRAWITEM: u32 = 0x002B;
pub const WM_SETICON: u32 = 0x0080;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_PRINTCLIENT: u32 = 0x0318;
pub const WM_DPICHANGED: u32 = 0x02E0;
pub const WM_USER: u32 = 0x0400;
pub const WM_APP: u32 = 0x8000;

pub const BN_CLICKED: u32 = 0;
pub const BN_DOUBLECLICKED: u32 = 5;
pub const CBN_SELCHANGE: u32 = 1;
pub const IDOK: i32 = 1;
pub const IDCANCEL: i32 = 2;

pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CB_RESETCONTENT: u32 = 0x014B;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const CB_SETMINVISIBLE: u32 = 0x1701;
pub const CB_SETCUEBANNER: u32 = 0x1703;
pub const EM_LIMITTEXT: u32 = 0x00C5;
pub const EM_SETCUEBANNER: u32 = 0x1501;

pub const IPM_CLEARADDRESS: u32 = WM_USER + 100;
pub const IPM_SETADDRESS: u32 = WM_USER + 101;
pub const IPM_GETADDRESS: u32 = WM_USER + 102;

// ---- 스타일 ----
pub const WS_OVERLAPPED: u32 = 0;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_MINIMIZEBOX: u32 = 0x0002_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const BS_OWNERDRAW: u32 = 0x0000_000B;
pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const ES_AUTOHSCROLL: u32 = 0x0080;
pub const CS_HREDRAW: u32 = 0x0002;
pub const CS_VREDRAW: u32 = 0x0001;
pub const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;

// ---- 그리기 ----
pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_RIGHT: u32 = 0x0002;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_WORDBREAK: u32 = 0x0010;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_NOPREFIX: u32 = 0x0800;
pub const DT_END_ELLIPSIS: u32 = 0x8000;
pub const ODS_SELECTED: u32 = 0x0001;
pub const ODS_DISABLED: u32 = 0x0004;
pub const ODS_FOCUS: u32 = 0x0010;
pub const TRANSPARENT: i32 = 1;
pub const SRCCOPY: u32 = 0x00CC_0020;
pub const NULL_PEN: i32 = 8;
pub const PS_SOLID: i32 = 0;
pub const FW_NORMAL: i32 = 400;
pub const FW_SEMIBOLD: i32 = 600;
pub const FW_BOLD: i32 = 700;
pub const DEFAULT_CHARSET: u32 = 1;
pub const CLEARTYPE_QUALITY: u32 = 5;

// ---- 기타 ----
pub const SW_HIDE: i32 = 0;
pub const SW_SHOW: i32 = 5;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const RDW_INVALIDATE: u32 = 0x0001;
pub const RDW_ALLCHILDREN: u32 = 0x0080;
pub const MF_STRING: u32 = 0x0000;
pub const MF_CHECKED: u32 = 0x0008;
pub const MF_SEPARATOR: u32 = 0x0800;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const MB_YESNO: u32 = 0x0004;
pub const MB_ICONQUESTION: u32 = 0x0020;
pub const IDYES: i32 = 6;
pub const IMAGE_ICON: u32 = 1;
pub const LR_DEFAULTSIZE: u32 = 0x0040;
pub const LR_SHARED: u32 = 0x8000;
pub const ICON_SMALL: usize = 0;
pub const ICON_BIG: usize = 1;
pub const IDC_ARROW: usize = 32512;
pub const ICC_STANDARD_CLASSES: u32 = 0x4000;
pub const ICC_INTERNET_CLASSES: u32 = 0x0800;

/// 널 문자로 끝나는 UTF-16 문자열
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// RGB → COLORREF (0x00BBGGRR)
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; len.max(0) as usize + 1];
        let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..got.max(0) as usize])
    }
}

pub fn set_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        SetWindowTextW(hwnd, text.as_ptr());
    }
}
