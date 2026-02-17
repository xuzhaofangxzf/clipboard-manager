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
                key_code = Some(match key.to_uppercase().as_str() {
                    "V" => Code::KeyV,
                    "C" => Code::KeyC,
                    "SPACE" => Code::Space,
                    _ => anyhow::bail!("Unsupported key: {}", key),
                });
            }
        }
    }

    let code = key_code.ok_or_else(|| anyhow::anyhow!("No key specified in shortcut"))?;
    Ok(HotKey::new(Some(modifiers), code))
}
