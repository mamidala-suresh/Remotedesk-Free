pub mod capture;
pub mod encoder;
pub mod input;

pub use capture::{CapturedFrame, ScreenCapturer, WindowsScreenCapturer};
pub use encoder::{EncoderConfig, EncoderStats, FrameEncoder};
pub use input::{InputInjector, WindowsInputInjector};

/// Initializes Per-Monitor DPI Awareness V2 for Windows 10/11.
/// Ensures pixel-for-pixel capture accuracy and exact mouse coordinate mapping across all DPI scaling levels (100%, 125%, 150%, 200%).
pub fn init_process_dpi_awareness() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
        // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 is ((DPI_AWARENESS_CONTEXT)-4)
        let _ = SetProcessDpiAwarenessContext(-4isize as *mut std::ffi::c_void);
    }
}
