use alloc::format;
use log::debug;

use crate::{graphic::PixelColor, interrupt::apic::LocalAPICRegisters, task::schedule};

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
    let mut apic_id = LocalAPICRegisters::default().local_apic_id().id();

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
        Point(16, 17),
        &format!("APIC ID: {apic_id}"),
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    draw_str(
        Point(16, 33),
        "Mouse Event:",
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    draw_str(
        Point(16, 49),
        "Data: X = 0, Y = 0",
        PixelColor::Black,
        PixelColor::White,
        &mut body,
    );

    button.draw(10, 90, &body.area(), &mut body);

    let mut pressed = 0;
    let mut released = 0;
    let mut c_x = 0;
    let mut c_y = 0;
    loop {
        let current_id = LocalAPICRegisters::default().local_apic_id().id();
        if apic_id != current_id {
            apic_id = current_id;
            draw_str(
                Point(88, 17),
                &format!("{apic_id}"),
                PixelColor::Black,
                PixelColor::White,
                &mut body,
            );
        }
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
                        Point(120, 33),
                        &format!("{str:10}:{value:3}"),
                        PixelColor::Black,
                        PixelColor::White,
                        &mut body,
                    );
                    if x != c_x || y != c_y {
                        draw_str(
                            Point(88, 49),
                            &format!("{x:3}"),
                            PixelColor::Black,
                            PixelColor::White,
                            &mut body,
                        );
                        draw_str(
                            Point(144, 49),
                            &format!("{y:3}"),
                            PixelColor::Black,
                            PixelColor::White,
                            &mut body,
                        );
                        c_x = x;
                        c_y = y;
                    }
                }
                EventType::Window(e) => match e {
                    WindowEvent::Close => {
                        writer.close();
                        return;
                    }
                    _ => {}
                },
                _ => {}
            }
        } else {
            schedule();
        }
    }
}
