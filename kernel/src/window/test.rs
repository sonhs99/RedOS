use alloc::format;
use log::debug;

use crate::{graphic::PixelColor, task::schedule};

use super::{
    component::Button,
    draw::{draw_rect, draw_str, Point},
    event::{EventType, MouseEvent, WindowEvent},
    frame::WindowFrame,
    Drawable,
};

pub fn test_window() {
    let width = 500;
    let height = 200;
    let mut writer = WindowFrame::new(width, height, "Hello, World!");
    let mut body = writer.body();
    let id = writer.window_id();

    let mut button = Button::new_default(width - 24, height - 104, 2, 0);

    draw_rect(
        Point(10, 8),
        Point(width - 10, 80),
        PixelColor::Black,
        false,
        &mut body,
    );
    draw_rect(
        Point(11, 9),
        Point(width - 11, 79),
        PixelColor::Black,
        false,
        &mut body,
    );

    draw_str(
        Point(20, 4),
        &format!("GUI Information Window[Window ID: {id:#010X}]"),
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    draw_str(
        Point(16, 32),
        "Mouse Event:",
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    draw_str(
        Point(16, 48),
        "Data: X = 0, Y = 0",
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    button.draw(10, 90, &body.area(), &mut body);

    let mut pressed = 0;
    let mut released = 0;
    loop {
        if let Some(event) = writer.pop_event() {
            match event.event() {
                EventType::Mouse(e, x, y) => {
                    let str = match e {
                        MouseEvent::Move => "Move",
                        MouseEvent::Pressed(_) => "Pressed",
                        MouseEvent::Released(_) => "Released",
                    };
                    let value = match e {
                        MouseEvent::Move => 0,
                        MouseEvent::Pressed(id) => {
                            pressed += 1;
                            if id == 0 && button.area(Point(10, 90)).is_in(x, y) {
                                button.update_bg(PixelColor(79, 204, 11));
                                button.draw(10, 90, &body.area(), &mut body);
                            }
                            pressed
                        }
                        MouseEvent::Released(id) => {
                            released += 1;
                            if id == 0 {
                                button.update_bg(PixelColor::White);
                                button.draw(10, 90, &body.area(), &mut body);
                            }
                            released
                        }
                    };
                    draw_str(
                        Point(16, 32),
                        &format!("Mouse Event: {str:10}:{value:3}"),
                        PixelColor::Black,
                        PixelColor::White,
                        &mut body,
                    );
                    draw_str(
                        Point(16, 48),
                        &format!("Data: X = {x:3}, Y = {y:3}"),
                        PixelColor::Black,
                        PixelColor::White,
                        &mut body,
                    );
                }
                EventType::Window(e) => {
                    if let WindowEvent::Close = e {
                        writer.close();
                        return;
                    }
                }
                _ => {}
            }
        } else {
            schedule();
        }
    }
}
