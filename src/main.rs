use scratchway::prelude::*;

mod app;
mod shared;

fn main() -> std::io::Result<()> {
    let Some(rom_path) = std::env::args().skip(1).next() else {
        scratchway::log!(ERR, "No rom provided");
        return Ok(())
    };

    let conn = Connection::connect()?;
    let wl_display = conn.display();
    let wl_registry = wl_display.get_registry(conn.writer());

    let mut app = app::App::new(wl_registry, wl_display);

    conn.roundtrip(&mut app)?;
    conn.dispatch_events(&mut app)?;

    if let Err(err) = app.init(&conn, &rom_path) {
        scratchway::log!(ERR, "{}", err);
        return Ok(())
    }

    app.enter_loop(&conn)?;
    // while !app.exit {
    //     conn.dispatch_events(&mut app)?;
    // }

    Ok(())
}
