use remotedesk_core::protocol::{KeyModifiers, MouseButton};

/// Input injection interface for remote control.
pub trait InputInjector: Send + Sync {
    fn mouse_move(&mut self, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String>;
    fn mouse_down(&mut self, button: MouseButton, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String>;
    fn mouse_up(&mut self, button: MouseButton, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String>;
    fn mouse_wheel(&mut self, delta_x: i32, delta_y: i32) -> Result<(), String>;
    fn key_down(&mut self, key_code: u32, scan_code: u32, modifiers: KeyModifiers) -> Result<(), String>;
    fn key_up(&mut self, key_code: u32, scan_code: u32, modifiers: KeyModifiers) -> Result<(), String>;
    fn text_input(&mut self, text: &str) -> Result<(), String>;
    fn execute_system_action(&mut self, action: &str) -> Result<(), String> {
        let _ = action;
        Ok(())
    }
}

/// Windows Input Injector using Win32 SendInput.
#[derive(Default)]
pub struct WindowsInputInjector;

impl WindowsInputInjector {
    pub fn new() -> Self {
        Self
    }
}

impl InputInjector for WindowsInputInjector {
    fn mouse_move(&mut self, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
                MOUSEINPUT,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN};

            let host_w = unsafe { GetSystemMetrics(SM_CXSCREEN) as f64 };
            let host_h = unsafe { GetSystemMetrics(SM_CYSCREEN) as f64 };

            if host_w <= 0.0 || host_h <= 0.0 {
                return Err("Invalid host screen metrics".to_string());
            }

            // Map incoming canvas coordinates (sw, sh) to host physical screen coordinates
            let phys_x = if screen_w > 0 {
                ((x as f64 * host_w) / screen_w as f64).round() as i32
            } else {
                x as i32
            };
            let phys_y = if screen_h > 0 {
                ((y as f64 * host_h) / screen_h as f64).round() as i32
            } else {
                y as i32
            };

            let norm_x = ((phys_x as f64 * 65535.0) / (host_w - 1.0).max(1.0)).round() as i32;
            let norm_y = ((phys_y as f64 * 65535.0) / (host_h - 1.0).max(1.0)).round() as i32;

            unsafe {
                SetCursorPos(phys_x, phys_y);
                let mut input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: norm_x,
                            dy: norm_y,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };

                let res = SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                if res != 1 {
                    return Err("SendInput mouse_move failed".to_string());
                }
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (x, y, screen_w, screen_h);
            Ok(())
        }
    }

    fn mouse_down(&mut self, button: MouseButton, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
                MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_RIGHTDOWN, MOUSEINPUT,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN};

            let host_w = unsafe { GetSystemMetrics(SM_CXSCREEN) as f64 };
            let host_h = unsafe { GetSystemMetrics(SM_CYSCREEN) as f64 };

            let phys_x = if screen_w > 0 && host_w > 0.0 {
                ((x as f64 * host_w) / screen_w as f64).round() as i32
            } else {
                x as i32
            };
            let phys_y = if screen_h > 0 && host_h > 0.0 {
                ((y as f64 * host_h) / screen_h as f64).round() as i32
            } else {
                y as i32
            };

            let flag = match button {
                MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
            };

            let norm_x = if host_w > 1.0 { ((phys_x as f64 * 65535.0) / (host_w - 1.0)).round() as i32 } else { 0 };
            let norm_y = if host_h > 1.0 { ((phys_y as f64 * 65535.0) / (host_h - 1.0)).round() as i32 } else { 0 };

            unsafe {
                SetCursorPos(phys_x, phys_y);
                let mut input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: norm_x,
                            dy: norm_y,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_ABSOLUTE | flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (button, x, y, screen_w, screen_h);
            Ok(())
        }
    }

    fn mouse_up(&mut self, button: MouseButton, x: u32, y: u32, screen_w: u32, screen_h: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTUP,
                MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN};

            let host_w = unsafe { GetSystemMetrics(SM_CXSCREEN) as f64 };
            let host_h = unsafe { GetSystemMetrics(SM_CYSCREEN) as f64 };

            let phys_x = if screen_w > 0 && host_w > 0.0 {
                ((x as f64 * host_w) / screen_w as f64).round() as i32
            } else {
                x as i32
            };
            let phys_y = if screen_h > 0 && host_h > 0.0 {
                ((y as f64 * host_h) / screen_h as f64).round() as i32
            } else {
                y as i32
            };

            let flag = match button {
                MouseButton::Left => MOUSEEVENTF_LEFTUP,
                MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
            };

            let norm_x = if host_w > 1.0 { ((phys_x as f64 * 65535.0) / (host_w - 1.0)).round() as i32 } else { 0 };
            let norm_y = if host_h > 1.0 { ((phys_y as f64 * 65535.0) / (host_h - 1.0)).round() as i32 } else { 0 };

            unsafe {
                SetCursorPos(phys_x, phys_y);
                let mut input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: norm_x,
                            dy: norm_y,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_ABSOLUTE | flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (button, x, y, screen_w, screen_h);
            Ok(())
        }
    }

    fn mouse_wheel(&mut self, _delta_x: i32, delta_y: i32) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT,
            };

            unsafe {
                let mut input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: (delta_y * 120) as u32,
                            dwFlags: MOUSEEVENTF_WHEEL,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = delta_y;
            Ok(())
        }
    }

    fn key_down(&mut self, key_code: u32, scan_code: u32, _modifiers: KeyModifiers) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYEVENTF_EXTENDEDKEY, MAPVK_VK_TO_VSC,
            };

            let is_ext = matches!(key_code, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D | 0x6F | 0x90 | 0xA3 | 0xA5);
            let ext_flag = if is_ext { KEYEVENTF_EXTENDEDKEY } else { 0 };
            let sc = if scan_code > 0 {
                scan_code as u16
            } else {
                unsafe { MapVirtualKeyW(key_code, MAPVK_VK_TO_VSC) as u16 }
            };

            unsafe {
                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key_code as u16,
                            wScan: sc,
                            dwFlags: ext_flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (key_code, scan_code);
            Ok(())
        }
    }

    fn key_up(&mut self, key_code: u32, scan_code: u32, _modifiers: KeyModifiers) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
            };

            let is_ext = matches!(key_code, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D | 0x6F | 0x90 | 0xA3 | 0xA5);
            let ext_flag = if is_ext { KEYEVENTF_EXTENDEDKEY } else { 0 };
            let sc = if scan_code > 0 {
                scan_code as u16
            } else {
                unsafe { MapVirtualKeyW(key_code, MAPVK_VK_TO_VSC) as u16 }
            };

            unsafe {
                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key_code as u16,
                            wScan: sc,
                            dwFlags: KEYEVENTF_KEYUP | ext_flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (key_code, scan_code);
            Ok(())
        }
    }

    fn text_input(&mut self, text: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
                KEYEVENTF_UNICODE,
            };

            for ch in text.encode_utf16() {
                unsafe {
                    let mut inputs = [
                        INPUT {
                            r#type: INPUT_KEYBOARD,
                            Anonymous: INPUT_0 {
                                ki: KEYBDINPUT {
                                    wVk: 0,
                                    wScan: ch,
                                    dwFlags: KEYEVENTF_UNICODE,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        },
                        INPUT {
                            r#type: INPUT_KEYBOARD,
                            Anonymous: INPUT_0 {
                                ki: KEYBDINPUT {
                                    wVk: 0,
                                    wScan: ch,
                                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        },
                    ];
                    SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
                }
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = text;
            Ok(())
        }
    }

    fn execute_system_action(&mut self, action: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
            };

            const VK_CONTROL: u16 = 0x11;
            const VK_MENU: u16 = 0x12; // Alt
            const VK_SHIFT: u16 = 0x10;
            const VK_ESCAPE: u16 = 0x1B;
            const VK_DELETE: u16 = 0x2E;
            const VK_LWIN: u16 = 0x5B;
            const VK_D: u16 = 0x44;
            const VK_E: u16 = 0x45;
            const VK_I: u16 = 0x49;
            const VK_L: u16 = 0x4C;
            const VK_R: u16 = 0x52;
            const VK_S: u16 = 0x53;
            const VK_X: u16 = 0x58;
            const VK_C: u16 = 0x43;
            const VK_V: u16 = 0x56;
            const VK_Z: u16 = 0x5A;
            const VK_Y: u16 = 0x59;
            const VK_A: u16 = 0x41;
            const VK_F4: u16 = 0x73;
            const VK_TAB: u16 = 0x09;

            let send_key = |vk: u16, key_up: bool| {
                let is_ext = matches!(vk, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D | 0x6F | 0x90 | 0xA3 | 0xA5);
                let ext_flag = if is_ext { KEYEVENTF_EXTENDEDKEY } else { 0 };
                let dw_flags = if key_up { KEYEVENTF_KEYUP | ext_flag } else { ext_flag };
                let sc = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16 };
                unsafe {
                    let mut input = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: vk,
                                wScan: sc,
                                dwFlags: dw_flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                }
            };

            let send_combo = |keys: &[u16]| {
                for &k in keys {
                    send_key(k, false);
                }
                // Short dwell delay so Windows Explorer shell hook registers the hotkey
                std::thread::sleep(std::time::Duration::from_millis(20));
                for &k in keys.iter().rev() {
                    send_key(k, true);
                }
            };

            match action {
                "task_manager" => {
                    send_combo(&[VK_CONTROL, VK_SHIFT, VK_ESCAPE]);
                    let _ = std::process::Command::new("taskmgr.exe").spawn();
                }
                "win_key" | "start_menu" => {
                    send_combo(&[VK_LWIN]);
                }
                "show_desktop" => {
                    send_combo(&[VK_LWIN, VK_D]);
                }
                "run_dialog" => {
                    send_combo(&[VK_LWIN, VK_R]);
                }
                "explorer" => {
                    send_combo(&[VK_LWIN, VK_E]);
                }
                "settings" => {
                    send_combo(&[VK_LWIN, VK_I]);
                }
                "quick_link_menu" => {
                    send_combo(&[VK_LWIN, VK_X]);
                }
                "search" => {
                    send_combo(&[VK_LWIN, VK_S]);
                }
                "lock_workstation" => {
                    send_combo(&[VK_LWIN, VK_L]);
                }
                "alt_tab" => {
                    send_combo(&[VK_MENU, VK_TAB]);
                }
                "win_tab" => {
                    send_combo(&[VK_LWIN, VK_TAB]);
                }
                "alt_f4" => {
                    send_combo(&[VK_MENU, VK_F4]);
                }
                "copy" => {
                    send_combo(&[VK_CONTROL, VK_C]);
                }
                "paste" => {
                    send_combo(&[VK_CONTROL, VK_V]);
                }
                "cut" => {
                    send_combo(&[VK_CONTROL, VK_X]);
                }
                "select_all" => {
                    send_combo(&[VK_CONTROL, VK_A]);
                }
                "undo" => {
                    send_combo(&[VK_CONTROL, VK_Z]);
                }
                "redo" => {
                    send_combo(&[VK_CONTROL, VK_Y]);
                }
                "ctrl_alt_del" => {
                    send_combo(&[VK_CONTROL, VK_MENU, VK_DELETE]);
                    let _ = std::process::Command::new("taskmgr.exe").spawn();
                }
                _ => return Err(format!("Unknown system action: {}", action)),
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = action;
            Ok(())
        }
    }
}
