use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyboardInput {
    kysym: u32,
    state: u8,
}

impl App {
    #[rustfmt::skip]
    pub(crate) fn on_wlseat(&mut self, conn: &Connection, event: WlEvent<'_>) {
        let Some(wl_seat) = self.wl_seat.take() else {
            return;
        };
        match wl_seat.parse_event(conn.reader(), event) {
            wl_seat::Event::Capabilities { capabilities } => {
                if capabilities & wl_seat::CAPABILITY_KEYBOARD > 0 {
                    let wl_keyboard = wl_seat.get_keyboard(conn.writer());
                    self.add_callback(wl_keyboard.id(), Self::on_wlkeyboard);
                    self.wl_keyboard = Some(wl_keyboard);
                } else {
                    if let Some(wl_keyboard) = self.wl_keyboard.take() {
                        wl_keyboard.release(conn.writer());
                    }
                }
            },
            wl_seat::Event::Name { .. } => {},
        }
        self.wl_seat = Some(wl_seat)
    }

    #[rustfmt::skip]
    pub(crate) fn on_wlkeyboard(&mut self, conn: &Connection, event: WlEvent) {
        let Some(wl_keyboard) = self.wl_keyboard.as_ref() else {
            return;
        };
        match wl_keyboard.parse_event(conn.reader(), event) {
            wl_keyboard::Event::Enter { serial, surface, keys } => {
                // self.focused_surface = Some(surface);
            },
            wl_keyboard::Event::Key { serial, time, key, state } => {
                let pressed = state != wl_keyboard::KeyState::Released;
                let keys = &mut self.chip8.keys;
                match key {
                    // Escape
                    1 => self.exit = true,
                    // 1 
                    2 => keys[0x1] = pressed,
                    // 2
                    3 => keys[0x2] = pressed,
                    // 3
                    4 => keys[0x3] = pressed,
                    // 4
                    5 => keys[0xC] = pressed,

                    // Q
                    16 => keys[0x4] = pressed,
                    // W
                    17 => keys[0x5] = pressed,
                    // E
                    18 => keys[0x6] = pressed,
                    // R
                    19 => keys[0xD] = pressed,


                    // A
                    30 => keys[0x7] = pressed,
                    // S
                    31 => keys[0x8] = pressed,
                    // D
                    32 => keys[0x9] = pressed,
                    // F
                    33 => keys[0xE] = pressed,

                    // Z
                    44 => keys[0xA] = pressed,
                    // X
                    45 => keys[0x0] = pressed,
                    // E
                    46 => keys[0xB] = pressed,
                    // R
                    47 => keys[0xF] = pressed,
                    _ => {}
                }
            },
            wl_keyboard::Event::Leave { serial, surface } => {
            },
            wl_keyboard::Event::Keymap { format, fd, size } => {},
            wl_keyboard::Event::Modifiers { serial, mods_depressed, mods_latched, mods_locked, group } => {},
            wl_keyboard::Event::RepeatInfo { rate, delay } => {},
            _ => {}
        }
    }

    pub(crate) fn pass_on_mouse(&mut self, conn: &Connection, event: wl_keyboard::Event) {}
}
