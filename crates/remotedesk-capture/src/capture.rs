use remotedesk_core::protocol::MonitorInfo;
use std::sync::atomic::{AtomicU64, Ordering};

/// Captured screen frame containing raw RGBA pixel data.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub frame_id: u64,
    pub timestamp_ms: u64,
    pub monitor_idx: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub cursor_shape: String,
}

static FRAME_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Detects the semantic cursor type from an HCURSOR/HICON handle.
#[cfg(windows)]
pub fn detect_cursor_type(h_cursor: windows_sys::Win32::UI::WindowsAndMessaging::HCURSOR) -> &'static str {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetIconInfo, LoadCursorW, ICONINFO, IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND,
        IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE,
        IDC_SIZEWE, IDC_WAIT,
    };

    if h_cursor.is_null() {
        return "default";
    }

    unsafe {
        // Direct handle comparison against system stock cursors
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_IBEAM) {
            return "text";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_HAND) {
            return "pointer";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_ARROW) {
            return "default";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_WAIT) {
            return "wait";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_CROSS) {
            return "crosshair";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_SIZEALL) {
            return "move";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_SIZENS) {
            return "ns-resize";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_SIZEWE) {
            return "ew-resize";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_SIZENWSE) {
            return "nwse-resize";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_SIZENESW) {
            return "nesw-resize";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_NO) {
            return "not-allowed";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_HELP) {
            return "help";
        }
        if h_cursor == LoadCursorW(std::ptr::null_mut(), IDC_APPSTARTING) {
            return "progress";
        }

        // Secondary check via Hotspot analysis if application set a custom or wrapped cursor
        let mut ii: ICONINFO = std::mem::zeroed();
        if GetIconInfo(h_cursor, &mut ii) != 0 {
            if !ii.hbmMask.is_null() {
                windows_sys::Win32::Graphics::Gdi::DeleteObject(ii.hbmMask);
            }
            if !ii.hbmColor.is_null() {
                windows_sys::Win32::Graphics::Gdi::DeleteObject(ii.hbmColor);
            }
            // For I-beam text cursors, the hotspot is centered horizontally (xHotspot > 1) and vertically (yHotspot > 3)
            // whereas standard arrow cursors have xHotspot == 0 && yHotspot == 0
            if ii.xHotspot > 1 && ii.yHotspot > 3 {
                return "text";
            }
        }

        "default"
    }
}

/// Screen capture interface.
pub trait ScreenCapturer: Send + Sync {
    fn list_monitors(&self) -> Vec<MonitorInfo>;
    fn capture_monitor(&mut self, monitor_idx: u32) -> Result<CapturedFrame, String>;
}

/// Windows Desktop Screen Capturer using Win32 GDI Desktop Duplication/BitBlt.
pub struct WindowsScreenCapturer {
    monitors: Vec<MonitorInfo>,
    selected_monitor: u32,
}

impl WindowsScreenCapturer {
    pub fn new() -> Self {
        let mut capturer = Self {
            monitors: Vec::new(),
            selected_monitor: 0,
        };
        capturer.refresh_monitors();
        capturer
    }

    pub fn refresh_monitors(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SM_CMONITORS, SM_CXSCREEN, SM_CYSCREEN,
            };

            let count = unsafe { GetSystemMetrics(SM_CMONITORS) };
            let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
            let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;

            let mut list = Vec::new();
            if count <= 1 {
                list.push(MonitorInfo {
                    index: 0,
                    name: "Primary Display".to_string(),
                    width: if width > 0 { width } else { 1920 },
                    height: if height > 0 { height } else { 1080 },
                    is_primary: true,
                    dpi_scale: 100,
                });
            } else {
                for i in 0..count as u32 {
                    list.push(MonitorInfo {
                        index: i,
                        name: format!("Display {}", i + 1),
                        width: if width > 0 { width } else { 1920 },
                        height: if height > 0 { height } else { 1080 },
                        is_primary: i == 0,
                        dpi_scale: 100,
                    });
                }
            }
            self.monitors = list;
        }

        #[cfg(not(windows))]
        {
            self.monitors = vec![MonitorInfo {
                index: 0,
                name: "Virtual Display".to_string(),
                width: 1920,
                height: 1080,
                is_primary: true,
                dpi_scale: 100,
            }];
        }
    }
}

impl Default for WindowsScreenCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapturer for WindowsScreenCapturer {
    fn list_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.clone()
    }

    fn capture_monitor(&mut self, monitor_idx: u32) -> Result<CapturedFrame, String> {
        self.selected_monitor = monitor_idx;

        #[cfg(windows)]
        {
            use windows_sys::Win32::Graphics::Gdi::{
                BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
                GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
                CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DrawIconEx, GetCursorInfo, GetSystemMetrics, CURSORINFO, CURSOR_SHOWING, DI_NORMAL,
                SM_CXSCREEN, SM_CYSCREEN,
            };

            let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

            if width <= 0 || height <= 0 {
                return Err("Failed to query screen metrics".to_string());
            }

            let w = width as u32;
            let h = height as u32;

            unsafe {
                let hdc_screen = GetDC(std::ptr::null_mut());
                if hdc_screen.is_null() {
                    return Err("Failed to get desktop DC".to_string());
                }

                let hdc_mem = CreateCompatibleDC(hdc_screen);
                if hdc_mem.is_null() {
                    ReleaseDC(std::ptr::null_mut(), hdc_screen);
                    return Err("Failed to create memory DC".to_string());
                }

                let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
                if hbitmap.is_null() {
                    DeleteDC(hdc_mem);
                    ReleaseDC(std::ptr::null_mut(), hdc_screen);
                    return Err("Failed to create compatible bitmap".to_string());
                }

                let old_obj = SelectObject(hdc_mem, hbitmap);

                // Copy screen pixels to memory DC (including layered and accelerated windows)
                let blt_res = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY | CAPTUREBLT);
                if blt_res == 0 {
                    SelectObject(hdc_mem, old_obj);
                    DeleteObject(hbitmap);
                    DeleteDC(hdc_mem);
                    ReleaseDC(std::ptr::null_mut(), hdc_screen);
                    return Err("BitBlt screen copy failed".to_string());
                }

                // Render physical mouse cursor on top of the captured screen frame and detect active cursor shape
                let mut ci: CURSORINFO = std::mem::zeroed();
                ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
                let mut cursor_shape = "default".to_string();
                if GetCursorInfo(&mut ci) != 0 && (ci.flags & CURSOR_SHOWING) != 0 && !ci.hCursor.is_null() {
                    cursor_shape = detect_cursor_type(ci.hCursor).to_string();
                    DrawIconEx(
                        hdc_mem,
                        ci.ptScreenPos.x,
                        ci.ptScreenPos.y,
                        ci.hCursor,
                        0,
                        0,
                        0,
                        std::ptr::null_mut(),
                        DI_NORMAL,
                    );
                }

                let mut bmi: BITMAPINFO = std::mem::zeroed();
                bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = width;
                bmi.bmiHeader.biHeight = -height; // Top-down
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;

                let mut bgra_buf: Vec<u8> = vec![0u8; (w * h * 4) as usize];
                let get_bits = GetDIBits(
                    hdc_mem,
                    hbitmap,
                    0,
                    h,
                    bgra_buf.as_mut_ptr() as *mut _,
                    &mut bmi,
                    DIB_RGB_COLORS,
                );

                // Clean up GDI handles immediately
                SelectObject(hdc_mem, old_obj);
                DeleteObject(hbitmap);
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);

                if get_bits == 0 {
                    return Err("GetDIBits failed to extract pixel data".to_string());
                }

                // Convert BGRA to RGBA in place for universal web/encoder compatibility
                let pixel_count = (w * h) as usize;
                for i in 0..pixel_count {
                    let b = bgra_buf[i * 4];
                    let r = bgra_buf[i * 4 + 2];
                    bgra_buf[i * 4] = r;
                    bgra_buf[i * 4 + 2] = b;
                }

                let frame_id = FRAME_COUNTER.fetch_add(1, Ordering::SeqCst);
                let timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                Ok(CapturedFrame {
                    frame_id,
                    timestamp_ms,
                    monitor_idx,
                    width: w,
                    height: h,
                    rgba: bgra_buf,
                    cursor_shape,
                })
            }
        }

        #[cfg(not(windows))]
        {
            let w = 800u32;
            let h = 600u32;
            let frame_id = FRAME_COUNTER.fetch_add(1, Ordering::SeqCst);
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            // Generate synthetic pattern for non-windows testing
            let mut rgba = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let idx = ((y * w + x) * 4) as usize;
                    rgba[idx] = (x % 256) as u8;
                    rgba[idx + 1] = (y % 256) as u8;
                    rgba[idx + 2] = 180;
                    rgba[idx + 3] = 255;
                }
            }

            Ok(CapturedFrame {
                frame_id,
                timestamp_ms,
                monitor_idx,
                width: w,
                height: h,
                rgba,
                cursor_shape: "default".to_string(),
            })
        }
    }
}
