//! Input handling for the compositor.

use anyhow::Result;
use tracing::debug;

use super::SurfaceManager;

/// Input event types.
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Keyboard key event.
    Key {
        keycode: u32,
        pressed: bool,
        modifiers: Modifiers,
    },
    /// Mouse button event.
    MouseButton {
        button: u32,
        pressed: bool,
        position: (f64, f64),
    },
    /// Mouse motion event.
    MouseMotion {
        position: (f64, f64),
        delta: (f64, f64),
    },
    /// Mouse scroll event.
    Scroll {
        delta: (f64, f64),
        position: (f64, f64),
    },
    /// Touch event.
    Touch {
        id: i32,
        phase: TouchPhase,
        position: (f64, f64),
    },
}

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Touch event phase.
#[derive(Debug, Clone, Copy)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// Handles input from various sources.
pub struct InputHandler {
    /// Current keyboard modifiers.
    modifiers: Modifiers,
    /// Current mouse position.
    mouse_position: (f64, f64),
    /// Pending events to process.
    pending_events: Vec<InputEvent>,
}

impl InputHandler {
    /// Create a new input handler.
    pub fn new() -> Self {
        Self {
            modifiers: Modifiers::default(),
            mouse_position: (0.0, 0.0),
            pending_events: Vec::new(),
        }
    }

    /// Queue an input event.
    pub fn queue_event(&mut self, event: InputEvent) {
        self.pending_events.push(event);
    }

    /// Process pending input events.
    pub fn process_events(&mut self, surfaces: &mut SurfaceManager) -> Result<()> {
        // Take ownership of pending events to avoid borrow conflicts
        let events = std::mem::take(&mut self.pending_events);
        for event in events {
            self.handle_event(event, surfaces)?;
        }
        Ok(())
    }

    /// Handle a single input event.
    fn handle_event(&mut self, event: InputEvent, surfaces: &mut SurfaceManager) -> Result<()> {
        match event {
            InputEvent::Key {
                keycode,
                pressed,
                modifiers,
            } => {
                self.modifiers = modifiers;
                debug!("Key event: {} {}", keycode, if pressed { "pressed" } else { "released" });

                // Handle compositor shortcuts
                if pressed {
                    self.handle_shortcut(keycode, surfaces)?;
                }
            }
            InputEvent::MouseButton {
                button,
                pressed,
                position,
            } => {
                self.mouse_position = position;
                debug!(
                    "Mouse button {} {} at ({}, {})",
                    button,
                    if pressed { "pressed" } else { "released" },
                    position.0,
                    position.1
                );
            }
            InputEvent::MouseMotion { position, delta: _ } => {
                self.mouse_position = position;
            }
            InputEvent::Scroll { delta, position } => {
                debug!("Scroll ({}, {}) at ({}, {})", delta.0, delta.1, position.0, position.1);
            }
            InputEvent::Touch { id, phase, position } => {
                debug!("Touch {} {:?} at ({}, {})", id, phase, position.0, position.1);
            }
        }
        Ok(())
    }

    /// Handle compositor keyboard shortcuts.
    fn handle_shortcut(&mut self, keycode: u32, _surfaces: &mut SurfaceManager) -> Result<()> {
        // Common shortcuts:
        // 1-9: Switch video source
        // Space: Pause/resume
        // Cmd+K: Command palette
        // Escape: Exit fullscreen

        match (keycode, self.modifiers.meta, self.modifiers.ctrl) {
            // Number keys for source switching
            (0x12..=0x1A, false, false) => {
                let source_num = keycode - 0x12 + 1;
                debug!("Switch to video source {}", source_num);
            }
            // Space for pause
            (0x31, false, false) => {
                debug!("Toggle pause");
            }
            // Cmd+K for command palette
            (0x28, true, false) => {
                debug!("Open command palette");
            }
            // Escape
            (0x35, false, false) => {
                debug!("Escape pressed");
            }
            _ => {}
        }

        Ok(())
    }

    /// Get current mouse position.
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    /// Get current modifiers.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
