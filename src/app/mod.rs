#![allow(unused)]

type Callback = fn(&mut App, &Connection, WlEvent<'_>);

#[derive(Debug)]
pub struct App {
    pub(crate) wl_display: wl_display::WlDisplay,
    pub(crate) wl_registry: wl_registry::WlRegistry,
    pub(crate) wl_compositor: Option<wl_compositor::WlCompositor>,
    pub(crate) viewporter: Option<WpViewporter>,

    pub(crate) wl_seat: Option<wl_seat::WlSeat>,
    pub(crate) wl_keyboard: Option<wl_keyboard::WlKeyboard>,

    pub(crate) wl_shm: Option<wl_shm::WlShm>,
    pub(crate) wl_buffer: Option<wl_buffer::WlBuffer>,
    pub(crate) shm: Option<shm::ShmData>,

    pub(crate) xdg_decoration_mgr: Option<ZxdgDecorationManagerV1>,
    pub(crate) xdg_base: Option<xdg_wm_base::XdgWmBase>,

    // actual Window
    pub(crate) base_surface: Option<wl_surface::WlSurface>,
    pub(crate) xdg_surface: Option<xdg_surface::XdgSurface>,
    pub(crate) xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub(crate) viewport: Option<WpViewport>,
    pub(crate) toplevel_deco: Option<ZxdgToplevelDecorationV1>,
    pub(crate) deco_mode: Option<zxdg_toplevel_decoration_v1::Mode>,
    pub(crate) deco_mode_changed: bool,
    pub(crate) configured: bool,

    pub(crate) win_height: i32,
    pub(crate) win_width: i32,

    pub(crate) chip8: chip8::Chip8,

    pub(crate) cbs: Vec<(u32, Callback)>,
    pub exit: bool,
}

impl App {
    pub fn new(wl_registry: wl_registry::WlRegistry, wl_display: wl_display::WlDisplay) -> Self {
        Self {
            cbs: vec![
                (wl_registry.id(), Self::on_wlregistry),
                (wl_display.id(), Self::on_wldisplay),
            ],
            wl_registry,
            wl_display,
            wl_compositor: None,
            wl_seat: None,
            wl_keyboard: None,
            wl_shm: None,
            exit: false,
            shm: None,
            xdg_base: None,
            base_surface: None,
            xdg_surface: None,
            xdg_toplevel: None,
            wl_buffer: None,
            configured: false,
            viewporter: None,
            viewport: None,
            win_height: 0,
            win_width: 0,
            chip8: chip8::Chip8::new(),
            xdg_decoration_mgr: None,
            toplevel_deco: None,
            deco_mode: None,
            deco_mode_changed: false,
        }
    }

    #[rustfmt::skip]
    fn on_wldisplay(&mut self, conn: &Connection, event: WlEvent<'_>) {
        match self.wl_display.parse_event(conn.reader(), event) {
            wl_display::Event::Error { object_id, code, message } => {
                log!(ERR, "Exprienced a fatel wayland error");
                log!(ERR, "object: {}, code: {}, message: {}", object_id, code, message);
                self.exit = true;
            },
            wl_display::Event::DeleteId { id } => {
                if let Some(i) = self.cbs.iter().position(|(o_id, _)| *o_id == id) {
                    let f = self.cbs.swap_remove(i);
                    log!(DEBUG, "Removed callback for id {}", f.0);
                }
            },
        }
    }

    #[rustfmt::skip]
    fn on_wlregistry(&mut self, conn: &Connection, event: WlEvent<'_>) {
        match self.wl_registry.parse_event(conn.reader(), event) {
            wl_registry::Event::Global { name, interface, version } => {
                match interface {
                    wl_shm::WlShm::INTERFACE => { // formats events are pointless for us
                        self.wl_shm = Some(self.wl_registry.bind(conn.writer(), name, interface, version));
                    },
                    wl_compositor::WlCompositor::INTERFACE => {
                        self.wl_compositor = Some(self.wl_registry.bind(conn.writer(), name, interface, version));
                    },
                    wl_seat::WlSeat::INTERFACE => {
                        let wl_seat = self.wl_registry.bind::<wl_seat::WlSeat>(conn.writer(), name, interface, version);
                        self.add_callback(wl_seat.id(), Self::on_wlseat);
                        self.wl_seat = Some(wl_seat);
                    },
                    xdg_wm_base::XdgWmBase::INTERFACE => {
                        let xdg_base = self.wl_registry.bind::<xdg_wm_base::XdgWmBase>(conn.writer(), name, interface, version);
                        self.add_callback(xdg_base.id(), Self::on_xdgbase);
                        self.xdg_base = Some(xdg_base);
                    }
                    WpViewporter::INTERFACE => {
                        self.viewporter = Some(self.wl_registry.bind(conn.writer(), name, interface, version));
                    }
                    ZxdgDecorationManagerV1::INTERFACE => {
                        self.xdg_decoration_mgr = Some(self.wl_registry.bind(conn.writer(), name, interface, version));
                    }
                    _ => {}
                }
            },
            wl_registry::Event::GlobalRemove { name } => {
            },
        }
    }

    pub fn init(&mut self, conn: &Connection, rom_path: &str) -> Result<(), &'static str> {
        if let Err(e) = self.chip8.load(rom_path) {
            log!(ERR, "'{}' - {}", rom_path, e);
            return Err("Failed to load rom");
        }

        let wl_compositor = self
            .wl_compositor
            .as_ref()
            .ok_or("wl_compositor is unavailable")?;
        let viewporter = self
            .viewporter
            .as_ref()
            .ok_or("Viewporter is unavailable")?;
        let wl_shm = self.wl_shm.as_ref().ok_or("wl_shm is unavailable")?;
        let xdg_base = self.xdg_base.as_ref().ok_or("xdg_base is unavailable")?;

        // Rust please let us have access to err in let-else syntax,
        // just like Zig
        let size = chip8::WIDTH * chip8::SCALE * chip8::HEIGHT * chip8::SCALE * 4;
        let (shm, fd) = match shm::ShmData::new(size) {
            Ok(d) => d,
            Err(e) => {
                log!(ERR, "{}", e);
                return Err("Couldn't allocate shared memoery");
            }
        };

        let pool = wl_shm.create_pool(conn.writer(), fd, size as i32);
        let wl_buffer = pool.create_buffer(
            conn.writer(),
            0,
            (chip8::SCALE * chip8::WIDTH) as i32,
            (chip8::SCALE * chip8::HEIGHT) as i32,
            (chip8::SCALE * chip8::WIDTH) as i32 * 4,
            wl_shm::Format::Xrgb8888 as u32,
        );
        pool.destroy(conn.writer());

        let wl_surface = wl_compositor.create_surface(conn.writer());
        let viewport = viewporter.get_viewport(conn.writer(), &wl_surface);
        let xdg_surface = xdg_base.get_xdg_surface(conn.writer(), &wl_surface);
        let xdg_toplevel = xdg_surface.get_toplevel(conn.writer());

        if let Some(deco_mgr) = self.xdg_decoration_mgr.as_ref() {
            let toplevel_deco = deco_mgr.get_toplevel_decoration(conn.writer(), &xdg_toplevel);
            let mode = zxdg_toplevel_decoration_v1::Mode::ServerSide;
            toplevel_deco.set_mode(conn.writer(), mode as u32);
            self.add_callback(toplevel_deco.id(), Self::on_xdgtoplevel_decoration);
            self.toplevel_deco = Some(toplevel_deco);
            self.deco_mode = Some(mode);
        }

        xdg_toplevel.set_title(conn.writer(), "eightpotatochips");
        xdg_toplevel.set_app_id(conn.writer(), "github.evillary.eightpotatochips");
        self.win_height = (chip8::SCALE * chip8::HEIGHT) as i32;
        self.win_width = (chip8::SCALE * chip8::WIDTH) as i32;
        xdg_toplevel.set_min_size(conn.writer(), self.win_width, self.win_height);
        xdg_toplevel.set_max_size(conn.writer(), self.win_width, self.win_height);
        viewport.set_destination(conn.writer(), self.win_width, self.win_height);
        wl_surface.commit(conn.writer());

        self.add_callback(xdg_toplevel.id(), Self::on_xdgtoplevel);
        self.add_callback(wl_surface.id(), Self::on_wlsurface);
        self.add_callback(wl_buffer.id(), Self::on_wlbuffer);
        self.add_callback(xdg_surface.id(), Self::on_xdgsurface);

        self.wl_buffer = Some(wl_buffer);
        self.base_surface = Some(wl_surface);
        self.xdg_toplevel = Some(xdg_toplevel);
        self.xdg_surface = Some(xdg_surface);
        self.shm = Some(shm);
        self.viewport = Some(viewport);

        Ok(())
    }

    pub fn enter_loop(&mut self, conn: &Connection) -> std::io::Result<()> {
        let ticker_fd = Ticker::new(24)?;
        let mut pollfds = [
            libc::pollfd {
                fd: conn.display_fd(),
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
            libc::pollfd {
                fd: ticker_fd.as_raw_fd(),
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
        ];

        while !self.exit {
            conn.flush()?;
            let ret = unsafe {
                syscall(libc::poll(
                    pollfds.as_mut_ptr().cast(),
                    pollfds.len() as u64, // ??
                    -1,
                ))
            }?;

            for pollfd in pollfds {
                if pollfd.revents & libc::POLLHUP > 0 {
                    log!(ERR, "{} fd hung up", pollfd.fd);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "fd hung up",
                    ));
                }
            }

            // tick timer
            if pollfds[1].revents & libc::POLLIN > 0 {
                let tickles = ticker_fd.read_timer()?;
                // in case some ticks are missed
                for _ in 0..tickles {
                    self.chip8.cycle();
                }
            }

            // wl_display
            if pollfds[0].revents & libc::POLLIN > 0 {
                match conn.dispatch_events(self) {
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => {}
                        _ => return Err(e),
                    },
                    Ok(()) => {}
                };
            }

            if self.chip8.need_redraw {
                self.redraw(conn);
            }
        }
        Ok(())
    }

    pub fn redraw(&mut self, conn: &Connection) {
        if !self.configured {
            return;
        }
        let Some(shm) = self.shm.as_mut() else {
            return;
        };
        let Some(wl_surface) = self.base_surface.as_ref() else {
            return;
        };

        // mutli thread this shit??
        let pixels_out = shm.as_slice_mut::<u32>();
        for (y, row) in pixels_out.chunks_mut(self.win_width as usize).enumerate() {
            let src_y = y / chip8::SCALE;
            for (x, pix) in row.iter_mut().enumerate() {
                let src_x = x / chip8::SCALE;
                if self.chip8.framebuffer[src_y * chip8::WIDTH + src_x] != 0 {
                    *pix = 0x79b67b;
                } else {
                    *pix = 0x000000;
                }
            }
        }
        // for (cell, out_p) in self.chip8.framebuffer.iter().zip(pixels_out) {
        //     if *cell > 0 {
        //         *out_p = 0x79b67b;
        //     } else {
        //         *out_p = 0x000000;
        //         // *out_p = 0x141618;
        //     }
        // }
        wl_surface.attach(conn.writer(), self.wl_buffer.as_ref(), 0, 0);
        wl_surface.damage_buffer(conn.writer(), 0, 0, i32::MAX, i32::MAX);
        wl_surface.commit(conn.writer());
        self.chip8.need_redraw = false;
    }

    #[rustfmt::skip]
    fn on_wlsurface(&mut self, conn: &Connection, event: WlEvent<'_>) {
        let Some(wl_surface) = self.base_surface.as_ref() else {
            return;
        };
        match wl_surface.parse_event(conn.reader(), event) {
            _ => {}
        }
    }

    #[rustfmt::skip]
    fn on_wlbuffer(&mut self, conn: &Connection, event: WlEvent<'_>) {
        let Some(wl_buffer) = self.wl_buffer.as_ref() else {
            return;
        };
        match wl_buffer.parse_event(conn.reader(), event) {
            _ => {}
        }
    }

    fn add_callback(&mut self, id: u32, cb: Callback) {
        self.cbs.push((id, cb));
    }
}

impl State for App {
    fn handle_event(&mut self, conn: &Connection, event: WlEvent<'_>) {
        let ev_id = event.header.id;
        let ev_op = event.header.opcode;
        if let Some((_, cb)) = self.cbs.iter().find(|(id, _)| *id == ev_id) {
            cb(self, conn, event);
        } else {
            log!(WARNING, "Discarded event from id: {}, op: {}", ev_id, ev_op);
        }
    }
}

use std::os::fd::AsRawFd;

use crate::shared::syscall;
use crate::shared::shm;
use crate::shared::ticker::Ticker;
use scr_protocols::viewporter::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};
use scr_protocols::xdg_shell::*;
use scr_protocols::xdg_decoration_unstable_v1::{*, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1};
use scratchway::log;
use scratchway::prelude::*;
use scratchway::wayland::*;

mod chip8;
mod input;
mod xdg;
