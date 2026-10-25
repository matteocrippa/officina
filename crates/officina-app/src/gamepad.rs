use egui::{Context, Event, Key};

pub struct GamepadState {
    pub gilrs: Option<gilrs::Gilrs>,
    pub connected: bool,
    pub status: String,
}

impl GamepadState {
    pub fn new() -> Self {
        match gilrs::Gilrs::new() {
            Ok(g) => Self {
                gilrs: Some(g),
                connected: false,
                status: "gamepad ready".into(),
            },
            Err(e) => Self {
                gilrs: None,
                connected: false,
                status: format!("gilrs init failed: {e}"),
            },
        }
    }

    pub fn poll(&mut self, ctx: &Context) {
        let Some(g) = &mut self.gilrs else {
            return;
        };
        self.connected = g.gamepads().count() > 0;
        if !self.connected {
            self.status = "no gamepad".into();
            // still drain events
        } else {
            self.status = "gamepad connected".into();
        }
        while let Some(ev) = g.next_event() {
            use gilrs::EventType::*;
            match ev.event {
                ButtonPressed(gilrs::Button::DPadUp, _) => push_key(ctx, Key::ArrowUp),
                ButtonPressed(gilrs::Button::DPadDown, _) => push_key(ctx, Key::ArrowDown),
                ButtonPressed(gilrs::Button::DPadLeft, _) => push_key(ctx, Key::ArrowLeft),
                ButtonPressed(gilrs::Button::DPadRight, _) => push_key(ctx, Key::ArrowRight),
                ButtonPressed(gilrs::Button::South, _) => push_key(ctx, Key::Enter),
                ButtonPressed(gilrs::Button::East, _) => push_key(ctx, Key::Escape),
                ButtonPressed(gilrs::Button::LeftTrigger, _) => push_key(ctx, Key::PageUp),
                ButtonPressed(gilrs::Button::RightTrigger, _) => push_key(ctx, Key::PageDown),
                _ => {}
            }
        }
    }
}

fn push_key(ctx: &Context, key: Key) {
    ctx.input_mut(|i| {
        i.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        });
    });
}
