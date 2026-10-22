// Calendar page with a blue header and date grid, legible at tray sizes.
pub fn calendar_icon() -> tauri::image::Image<'static> {
    let mut rgba = vec![0u8; 32 * 32 * 4];
    for y in 0..32usize {
        for x in 0..32usize {
            let body = (3..29).contains(&x) && (5..29).contains(&y);
            let edge = body && (x == 3 || x == 28 || y == 28);
            let header = body && y < 12;
            let binding = ((9..12).contains(&x) || (20..23).contains(&x)) && (3..8).contains(&y);
            let cell = (15..25).contains(&y)
                && ((y - 15) % 6 < 3)
                && (8..25).contains(&x)
                && ((x - 8) % 6 < 3);
            let color = if binding {
                [58, 79, 100, 255]
            } else if header {
                [0, 120, 212, 255]
            } else if edge {
                [122, 149, 174, 255]
            } else if cell {
                [0, 95, 184, 255]
            } else if body {
                [248, 251, 255, 255]
            } else {
                [0, 0, 0, 0]
            };
            rgba[(y * 32 + x) * 4..(y * 32 + x + 1) * 4].copy_from_slice(&color);
        }
    }
    tauri::image::Image::new_owned(rgba, 32, 32)
}
