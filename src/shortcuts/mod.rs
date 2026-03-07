use anyhow::{Context, Result};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};

pub struct ShortcutManager {
    manager: GlobalHotKeyManager,
    current_hotkey: Option<(HotKey, u32)>,
}

impl ShortcutManager {
    pub fn new() -> Result<Self> {
        let manager =
            GlobalHotKeyManager::new().context("Failed to initialize GlobalHotKeyManager")?;
        Ok(Self {
            manager,
            current_hotkey: None,
        })
    }

    pub fn register(&mut self, shortcut_str: &str) -> Result<()> {
        // Parse shortcut string like "Cmd+Shift+V" or "Control+Shift+V"
        let hotkey = parse_shortcut(shortcut_str)?;

        // Unregister old one if exists
        if let Some((old_hotkey, _)) = self.current_hotkey {
            self.manager.unregister(old_hotkey)?;
        }

        // Register new one
        self.manager.register(hotkey)?;
        self.current_hotkey = Some((hotkey, hotkey.id()));

        Ok(())
    }

    pub fn setup_event_handler<F>(on_hotkey: F)
    where
        F: Fn(GlobalHotKeyEvent) + Send + 'static,
    {
        std::thread::spawn(move || {
            let receiver = GlobalHotKeyEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    on_hotkey(event);
                }
            }
        });
    }

    pub fn current_hotkey_id(&self) -> Option<u32> {
        self.current_hotkey.map(|(_, id)| id)
    }
}

fn parse_shortcut(s: &str) -> Result<HotKey> {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "cmd" | "command" | "meta" | "super" => modifiers.insert(Modifiers::META),
            "shift" => modifiers.insert(Modifiers::SHIFT),
            "alt" | "option" => modifiers.insert(Modifiers::ALT),
            "ctrl" | "control" => modifiers.insert(Modifiers::CONTROL),
            key => {
                // Try to parse as Code
                // This is a simplified version, ideally we'd map more keys
                key_code = Some(parse_key_code(key)?);
            }
        }
    }

    let code = key_code.ok_or_else(|| anyhow::anyhow!("No key specified in shortcut"))?;
    Ok(HotKey::new(Some(modifiers), code))
}

fn parse_key_code(key: &str) -> Result<Code> {
    let upper = key.trim().to_uppercase();
    let code = match upper.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "SPACE" => Code::Space,
        _ => anyhow::bail!("Unsupported key: {}", key),
    };
    Ok(code)
}
