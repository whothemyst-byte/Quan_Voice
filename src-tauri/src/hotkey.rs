use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("hotkey registration failed: {0}")]
    RegistrationFailed(String),
}

pub struct HotkeyManager {
    registered_key: Option<String>,
    listener_running: Option<Arc<AtomicBool>>,
    listener_thread: Option<JoinHandle<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            registered_key: None,
            listener_running: None,
            listener_thread: None,
        }
    }

    pub fn register_system_hotkey<F1, F2>(
        &mut self,
        key: &str,
        on_press: F1,
        on_release: F2,
    ) -> Result<(), HotkeyError>
    where
        F1: Fn() + Send + Sync + 'static,
        F2: Fn() + Send + Sync + 'static,
    {
        if key.trim().is_empty() {
            return Err(HotkeyError::RegistrationFailed(
                "empty hotkey is invalid".to_string(),
            ));
        }

        self.stop_listener();

        let vk = parse_virtual_key(key)?;
        let running = Arc::new(AtomicBool::new(true));
        let running_for_thread = Arc::clone(&running);
        let on_press = Arc::new(on_press);
        let on_release = Arc::new(on_release);

        let listener = thread::spawn(move || {
            let mut was_down = false;
            while running_for_thread.load(Ordering::Relaxed) {
                #[allow(unsafe_code)]
                let current_state = unsafe {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32)
                };
                let is_down = (current_state as u16 & 0x8000) != 0;

                if is_down && !was_down {
                    on_press();
                } else if !is_down && was_down {
                    on_release();
                }

                was_down = is_down;
                thread::sleep(Duration::from_millis(12));
            }
        });

        self.listener_running = Some(running);
        self.listener_thread = Some(listener);
        self.registered_key = Some(key.to_string());
        Ok(())
    }

    fn stop_listener(&mut self) {
        if let Some(flag) = &self.listener_running {
            flag.store(false, Ordering::Relaxed);
        }

        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }

        self.listener_running = None;
        self.registered_key = None;
    }

    pub fn unregister_system_hotkey(&mut self) {
        self.stop_listener();
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop_listener();
    }
}

fn parse_virtual_key(key: &str) -> Result<u16, HotkeyError> {
    let normalized = key.trim().to_ascii_lowercase();
    let code = match normalized.as_str() {
        "ctrl" | "control" | "lctrl" | "leftctrl" | "left_ctrl" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL
        }
        "rightctrl" | "right_ctrl" | "rctrl" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RCONTROL
        }
        "shift" | "lshift" | "leftshift" | "left_shift" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT
        }
        "rightshift" | "right_shift" | "rshift" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RSHIFT
        }
        "alt" | "menu" | "lalt" | "leftalt" | "left_alt" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MENU
        }
        "rightalt" | "right_alt" | "ralt" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RMENU
        }
        "tab" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_TAB,
        "space" | "spacebar" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SPACE,
        "enter" | "return" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN,
        "esc" | "escape" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE,
        "capslock" | "caps_lock" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CAPITAL
        }
        "backspace" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK,
        "delete" | "del" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_DELETE,
        "insert" | "ins" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_INSERT,
        "home" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_HOME,
        "end" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_END,
        "pageup" | "page_up" | "pgup" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_PRIOR
        }
        "pagedown" | "page_down" | "pgdn" => {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_NEXT
        }
        "up" | "arrowup" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_UP,
        "down" | "arrowdown" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_DOWN,
        "left" | "arrowleft" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_LEFT,
        "right" | "arrowright" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RIGHT,
        "f1" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F1,
        "f2" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F2,
        "f3" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F3,
        "f4" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4,
        "f5" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F5,
        "f6" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F6,
        "f7" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F7,
        "f8" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F8,
        "f9" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F9,
        "f10" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F10,
        "f11" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F11,
        "f12" => windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F12,
        _ => {
            if normalized.len() == 1 {
                let byte = normalized.as_bytes()[0];
                if byte.is_ascii_alphanumeric() {
                    return Ok(byte.to_ascii_uppercase() as u16);
                }
            }
            return Err(HotkeyError::RegistrationFailed(format!(
                "unsupported hotkey `{key}`"
            )));
        }
    };

    Ok(code as u16)
}

#[cfg(test)]
mod tests {
    use super::parse_virtual_key;

    #[test]
    fn parses_page_down_aliases() {
        let a = parse_virtual_key("PageDown").unwrap();
        let b = parse_virtual_key("pgdn").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parses_letter_keys_case_insensitive() {
        let a = parse_virtual_key("a").unwrap();
        let b = parse_virtual_key("A").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(parse_virtual_key("not-a-key").is_err());
    }
}
