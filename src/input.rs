use crate::math::vector::Vector2;

pub enum InputEvent {
    /// Rotate viewport about some pivot point, e.g. turntable rotation.
    DoViewportOrbit,
    /// The keys for doing the viewport orbit have just been released.
    FinishViewportOrbit,
}

pub struct InputState {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            mouse: MouseState::default(),
            keyboard: KeyboardState::default(),
        }
    }
}

impl InputState {
    pub fn get_input_events(&self) -> Vec<InputEvent> {
        let mut input_events = Vec::new();

        // DoViewportOrbit
        if self.mouse.lmb_pressed && self.keyboard.shift_pressed {
            input_events.push(InputEvent::DoViewportOrbit);
        }

        // FinishViewportOrbit TODO
        if self.mouse.lmb_released || self.keyboard.shift_released {
            input_events.push(InputEvent::FinishViewportOrbit);
        }

        input_events
    }

    pub fn reset_release_events(&mut self) {
        self.mouse.reset_release_events();
        self.keyboard.reset_release_events();
    }
}

pub struct MouseState {
    pub lmb_pressed: bool,
    /// True if the button has just been released. This should only be true for one pass
    /// through the event loop-- as soon as the released is processed and handled, it is
    /// set back to false.
    pub lmb_released: bool,
    pub cursor_pos_on_pressed: Option<Vector2>,
    pub curr_cursor_pos: Vector2,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            lmb_pressed: false,
            lmb_released: false,
            cursor_pos_on_pressed: None,
            curr_cursor_pos: Vector2::new(0., 0.),
        }
    }
}

impl MouseState {
    fn reset_release_events(&mut self) {
        self.lmb_released = false;
    }
}

pub struct KeyboardState {
    pub shift_pressed: bool,
    pub shift_released: bool,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            shift_pressed: false,
            shift_released: false,
        }
    }
}

impl KeyboardState {
    fn reset_release_events(&mut self) {
        self.shift_released = false;
    }
}
