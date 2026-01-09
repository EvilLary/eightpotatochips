use super::*;

impl App {
    pub(crate) fn on_xdgbase(&mut self, conn: &Connection, event: WlEvent) {
        let Some(xdg_base) = self.xdg_base.take() else {
            return;
        };
        match xdg_base.parse_event(conn.reader(), event) {
            xdg_wm_base::Event::Ping { serial } => {
                xdg_base.pong(conn.writer(), serial);
            }
        }
        self.xdg_base = Some(xdg_base);
    }

    pub(crate) fn on_xdgsurface(&mut self, conn: &Connection, event: WlEvent) {
        let Some(xdg_surface) = self.xdg_surface.take() else {
            return;
        };
        match xdg_surface.parse_event(conn.reader(), event) {
            xdg_surface::Event::Configure { serial } => {
                xdg_surface.ack_configure(conn.writer(), serial);
                if self.configured {
                    let wl_surface = self.base_surface.as_ref().unwrap();
                    if let Some(viewport) = self.viewport.as_ref() {
                        viewport.set_destination(conn.writer(), self.win_width, self.win_height);
                    }
                    xdg_surface.set_window_geometry(conn.writer(), 0, 0, self.win_width, self.win_height);
                    wl_surface.attach(conn.writer(), self.wl_buffer.as_ref(), 0, 0);
                    wl_surface.damage_buffer(conn.writer(), 0, 0, self.win_width, self.win_height);
                    wl_surface.commit(conn.writer());
                } else {
                    let wl_surface = self.base_surface.as_ref().unwrap();
                    if let Some(viewport) = self.viewport.as_ref() {
                        viewport.set_destination(conn.writer(), self.win_width, self.win_height);
                    }
                    wl_surface.attach(conn.writer(), self.wl_buffer.as_ref(), 0, 0);
                    wl_surface.commit(conn.writer());
                    self.configured = true;
                }
                if self.deco_mode_changed {
                    if let Some(toplevel_deco) = self.toplevel_deco.as_ref() {
                        toplevel_deco.set_mode(conn.writer(), *self.deco_mode.as_ref().unwrap() as u32);
                    }
                }
            }
        }
        self.xdg_surface = Some(xdg_surface);
    }

    #[rustfmt::skip]
    pub(crate) fn on_xdgtoplevel(&mut self, conn: &Connection, event: WlEvent) {
        let Some(xdg_toplevel) = self.xdg_toplevel.take() else {
            return;
        };

        match xdg_toplevel.parse_event(conn.reader(), event) {
            xdg_toplevel::Event::Configure { width, height, states } => {
                if height != 0 && width != 0 {
                    self.win_width = width;
                    self.win_height = height;
                }
            }
            xdg_toplevel::Event::Close => {
                self.exit = true;
            }
            _ => {}
        }

        self.xdg_toplevel = Some(xdg_toplevel);
    }

    #[rustfmt::skip]
    pub(crate) fn on_xdgtoplevel_decoration(&mut self, conn: &Connection, event: WlEvent) {
        let Some(toplevel_deco) = self.toplevel_deco.as_ref() else {
            return;
        };
        match toplevel_deco.parse_event(conn.reader(), event) {
            zxdg_toplevel_decoration_v1::Event::Configure { mode } => {
                let mode = u32_to_mode(mode);
                if self.deco_mode != Some(mode) {
                    self.deco_mode_changed = true;
                    self.deco_mode = Some(mode);
                }
            },
        }

    }
}

// TODO: do this in the scanner side
fn u32_to_mode(num: u32) -> zxdg_toplevel_decoration_v1::Mode {
    match num {
        1 => zxdg_toplevel_decoration_v1::Mode::ClientSide,
        2 => zxdg_toplevel_decoration_v1::Mode::ServerSide,
        _ => unreachable!()
    }
}
