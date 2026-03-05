use std::mem::size_of;

use thiserror::Error;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VK_BACK, VK_RETURN, VK_TAB,
};

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("cannot inject empty text")]
    EmptyText,
    #[error("text injection failed: {0}")]
    InjectionFailed(String),
}

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        Self
    }

    pub fn inject_text(&self, text: &str) -> Result<(), InjectError> {
        if text.trim().is_empty() {
            return Err(InjectError::EmptyText);
        }

        let inputs = build_inputs(text);
        if inputs.is_empty() {
            return Err(InjectError::EmptyText);
        }

        send_inputs_chunked(&inputs)
    }
}

fn build_inputs(text: &str) -> Vec<INPUT> {
    let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
    let mut saw_cr = false;

    for ch in text.chars() {
        if ch == '\r' {
            saw_cr = true;
            push_enter(&mut inputs);
            continue;
        }

        if ch == '\n' {
            if !saw_cr {
                push_enter(&mut inputs);
            }
            saw_cr = false;
            continue;
        }

        if ch == '\t' {
            saw_cr = false;
            push_tab(&mut inputs);
            continue;
        }

        if ch == '\u{8}' {
            saw_cr = false;
            push_backspace(&mut inputs);
            continue;
        }

        saw_cr = false;
        for unit in ch.encode_utf16(&mut [0; 2]).iter().copied() {
            push_unicode(&mut inputs, unit, false);
            push_unicode(&mut inputs, unit, true);
        }
    }

    inputs
}

fn push_unicode(inputs: &mut Vec<INPUT>, unit: u16, key_up: bool) {
    let mut flags = KEYEVENTF_UNICODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
}

fn push_enter(inputs: &mut Vec<INPUT>) {
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_RETURN,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_RETURN,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
}

fn push_tab(inputs: &mut Vec<INPUT>) {
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_TAB,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_TAB,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
}

fn push_backspace(inputs: &mut Vec<INPUT>) {
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_BACK,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_BACK,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
}

fn send_inputs_chunked(inputs: &[INPUT]) -> Result<(), InjectError> {
    const CHUNK_SIZE: usize = 256;
    let mut offset = 0usize;

    while offset < inputs.len() {
        let end = (offset + CHUNK_SIZE).min(inputs.len());
        let chunk = &inputs[offset..end];
        #[allow(unsafe_code)]
        let sent = unsafe {
            SendInput(
                chunk.len() as u32,
                chunk.as_ptr() as *const INPUT,
                size_of::<INPUT>() as i32,
            )
        };

        if sent != chunk.len() as u32 {
            let os_err = std::io::Error::last_os_error();
            return Err(InjectError::InjectionFailed(format!(
                "sent {sent}/{} events at chunk starting {offset} ({os_err})",
                chunk.len()
            )));
        }

        offset = end;
    }

    Ok(())
}
