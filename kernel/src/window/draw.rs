use alloc::vec;

use crate::{font::write_ascii, graphic::PixelColor, utility::abs};

use super::{request_update_by_id, Writable};

#[derive(Clone, Copy)]
pub struct Point(pub usize, pub usize);

pub fn draw_line(p1: Point, p2: Point, color: PixelColor, writer: &mut impl Writable) {
    let (start, end) = if p1.0 <= p2.0 { (p1, p2) } else { (p2, p1) };

    let dx = end.0 - start.0;
    let (neg, dy) = if end.1 >= start.1 {
        (false, end.1 - start.1)
    } else {
        (true, start.1 - end.1)
    };

    let mut error = 0;
    if dx > dy {
        let de = dy << 1;
        let mut y_i = 0;
        for x_i in 0..=dx {
            if neg {
                writer.write(start.0 + x_i, start.1 - y_i, color);
            } else {
                writer.write(start.0 + x_i, start.1 + y_i, color);
            }
            error += de;
            if error >= dx {
                y_i += 1;
                error = error - dx << 1;
            }
        }
    } else {
        let de = dx << 1;
        let mut x_i = 0;
        for y_i in 0..=dy {
            if neg {
                writer.write(start.0 + x_i, start.1 - y_i, color);
            } else {
                writer.write(start.0 + x_i, start.1 + y_i, color);
            }
            error += de;
            if error >= dy {
                x_i += 1;
                error = error - dy << 1;
            }
        }
    }
    if let Some(id) = writer.write_id() {
        request_update_by_id(id);
    }
}

pub fn draw_rect(p1: Point, p2: Point, color: PixelColor, fill: bool, writer: &mut impl Writable) {
    if !fill {
        draw_line(p1, Point(p1.0, p2.1), color, writer);
        draw_line(p1, Point(p2.0, p1.1), color, writer);
        draw_line(Point(p1.0, p2.1), p2, color, writer);
        draw_line(Point(p2.0, p1.1), p2, color, writer);
    } else {
        let (start, end) = if p1.0 <= p2.0 { (p1, p2) } else { (p2, p1) };
        let dx = end.0 - start.0;
        let dy = if end.1 >= start.1 {
            end.1 - start.1
        } else {
            start.1 - end.1
        };

        let buffer = vec![color.as_u32(); dx];
        for idx in 0..dy {
            writer.write_buf(start.0, start.1 + idx, &buffer);
        }
    }
    if let Some(id) = writer.write_id() {
        request_update_by_id(id);
    }
}

pub fn draw_circle(
    center: Point,
    radius: usize,
    color: PixelColor,
    fill: bool,
    writer: &mut impl Writable,
) {
    if !fill {
        writer.write(center.0 + radius, center.1, color);
        writer.write(center.0, center.1 + radius, color);
        writer.write(center.0 - radius, center.1, color);
        writer.write(center.0, center.1 - radius, color);
    } else {
        draw_line(
            Point(center.0 - radius, center.1),
            Point(center.0 + radius, center.1),
            color,
            writer,
        );
        draw_line(
            Point(center.0, center.1 - radius),
            Point(center.0, center.1 + radius),
            color,
            writer,
        );
    }

    let mut y_i = radius;
    let mut d = -(radius as isize);
    for x_i in 1..radius {
        if y_i > x_i {
            break;
        }
        d += 2 * x_i as isize - 1;
        if d >= 0 {
            y_i -= 1;
            d += 2 - 2 * y_i as isize;
        }

        if !fill {
            writer.write(center.0 + x_i, center.1 + y_i, color);
            writer.write(center.0 + y_i, center.1 + x_i, color);

            writer.write(center.0 - x_i, center.1 + y_i, color);
            writer.write(center.0 + y_i, center.1 - x_i, color);

            writer.write(center.0 - x_i, center.1 - y_i, color);
            writer.write(center.0 - y_i, center.1 - x_i, color);

            writer.write(center.0 + x_i, center.1 - y_i, color);
            writer.write(center.0 - y_i, center.1 + x_i, color);
        } else {
            let max = if x_i > y_i { x_i } else { y_i };
            let buffer = vec![color.as_u32(); max * 2];
            writer.write_buf(center.0 - x_i, center.1 + y_i, &buffer[..2 * x_i]);
            writer.write_buf(center.0 - x_i, center.1 - y_i, &buffer[..2 * x_i]);
            writer.write_buf(center.0 - y_i, center.1 + x_i, &buffer[..2 * y_i]);
            writer.write_buf(center.0 - y_i, center.1 - x_i, &buffer[..2 * y_i]);
        }
    }
    if let Some(id) = writer.write_id() {
        request_update_by_id(id);
    }
}

pub fn draw_str(
    offset: Point,
    str: &str,
    foreground: PixelColor,
    background: PixelColor,
    writer: &mut impl Writable,
) {
    let mut x = 0usize;
    let mut y = 0usize;
    for (idx, c) in str.bytes().enumerate() {
        if c >= 0x20 && c <= 0x7F {
            write_ascii(
                (x * 8 + offset.0) as u64,
                (y * 16 + offset.1) as u64,
                c,
                foreground,
                Some(background),
                writer,
            );
            x += 1;
        } else if c == b'\n' {
            y += 1;
            x = 0;
        }
    }
    if let Some(id) = writer.write_id() {
        request_update_by_id(id);
    }
}
