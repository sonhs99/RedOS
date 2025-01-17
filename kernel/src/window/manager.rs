use alloc::{collections::btree_map::BTreeMap, vec, vec::Vec};
use log::debug;

use crate::{collections::queue::Queue, utility::random};

use super::{
    curser::print_curser,
    draw::Point,
    event::{self, DestId, Event, EventType, UpdateEvent, EVENT_QUEUE_SIZE},
    writer::FrameBuffer,
    Area, BitmapDrawable, DrawBitmap, Drawable, Layer, Writable,
};

pub struct WindowManager {
    layers: BTreeMap<usize, Layer>,
    stack: Vec<usize>,
    event_queue: Queue<Vec<Event>>,
    global: FrameBuffer,
    resolution: (usize, usize),
    count: usize,
    mouse_x: usize,
    mouse_y: usize,
    update: bool,
    moving: bool,
}

impl WindowManager {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            layers: BTreeMap::new(),
            stack: Vec::new(),
            global: FrameBuffer::new(resolution.0, resolution.1),
            resolution,
            event_queue: Queue::new(vec![Event::default(); EVENT_QUEUE_SIZE]),
            count: 0,
            mouse_x: resolution.0 / 2,
            mouse_y: resolution.1 / 2,
            update: false,
            moving: false,
        }
    }

    pub const fn mouse(&self) -> Point {
        Point(self.mouse_x, self.mouse_y)
    }

    pub fn set_mouse(&mut self, new_pos: Point) {
        self.mouse_x = new_pos.0;
        self.mouse_y = new_pos.1;
    }

    pub const fn resolution(&self) -> (usize, usize) {
        self.resolution
    }

    pub const fn area(&self) -> Area {
        Area {
            x: 0,
            y: 0,
            width: self.resolution.0,
            height: self.resolution.1,
        }
    }

    pub fn set_moving(&mut self, flag: bool) {
        self.moving = flag;
    }

    pub const fn moving(&self) -> bool {
        self.moving
    }

    pub fn request_global_update(&mut self) {
        self.update = true;
    }

    pub fn new_layer(&mut self, width: usize, height: usize, relocatable: bool) -> &mut Layer {
        let (x, y) = (
            random() as usize % self.resolution.0,
            random() as usize % self.resolution.1,
        );
        self.new_layer_pos(x, y, width, height, relocatable)
    }

    pub fn new_layer_pos(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        relocatable: bool,
    ) -> &mut Layer {
        let id = self.count;
        self.count = self.count.wrapping_add(1);
        self.layers
            .insert(id, Layer::new(id, x, y, width, height, relocatable));
        self.stack.push(id);
        self.push_event(Event::new(
            DestId::One(id),
            EventType::Update(UpdateEvent::Id(id)),
        ));
        self.layers.get_mut(&id).expect("Not Found")
    }

    pub fn focus(&mut self, id: usize) {
        if !self.layers.contains_key(&id) {
            return;
        }
        if !self.get_layer(id).unwrap().relocatable {
            return;
        }
        for (idx, &window_id) in self.stack.iter().enumerate() {
            if window_id == id {
                self.stack.remove(idx);
                self.stack.push(id);
                self.get_layer(id).unwrap().need_update();
                return;
            }
        }
        // debug!("Top Layer id={id}");
    }

    pub fn visibility(&mut self, id: usize, visible: bool) {
        if !self.layers.contains_key(&id) {
            return;
        }

        if visible {
            if self
                .stack
                .iter()
                .enumerate()
                .find(|(idx, &x)| x == id)
                .is_none()
            {
                self.stack.push(id);
            }
        } else {
            if let Some((idx, _)) = self.stack.iter().enumerate().find(|(idx, &x)| x == id) {
                self.stack.remove(idx);
            }
        }
        self.get_layer(id).unwrap().need_update();
    }

    pub fn remove(&mut self, id: usize) {
        // debug!("[WINDOW] Window ID={id} Removed Start");
        for (idx, &window_id) in self.stack.iter().enumerate() {
            if window_id == id {
                self.stack.remove(idx);
                // debug!("stack removed");
                break;
            }
        }

        // self.layers.remove(&id);
        let mut list: Vec<usize> = self.layers.keys().map(|&id| id).collect();
        // list.sort();
        // debug!("Layer: {list:?}");
        // debug!("Stack: {:?}", self.stack);
        // debug!("[WINDOW] Window ID={id} Removed");
        self.update = true;
    }

    pub fn get_layer(&mut self, id: usize) -> Option<&mut Layer> {
        self.layers.get_mut(&id)
    }

    pub fn top_layer(&mut self) -> &mut Layer {
        let top_id = self.stack.last().unwrap();
        self.layers.get_mut(top_id).unwrap()
    }

    pub fn get_layer_id_from_point(&mut self, x: usize, y: usize) -> usize {
        for layer_id in self.stack.iter().rev() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            if layer.area().is_in(x, y) {
                return *layer_id;
            }
        }
        0
    }

    pub fn get_mouse(&self) -> (usize, usize) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn push_event(&mut self, event: Event) {
        if let EventType::Update(update_event) = event.event() {
            if let UpdateEvent::Id(id) = update_event {
                match self.get_layer(id) {
                    Some(layer) => {
                        if layer.update {
                            return;
                        }
                    }
                    None => return,
                };
            }
        }
        self.event_queue.enqueue(event);
    }

    pub fn pop_event(&mut self) -> Result<Event, ()> {
        self.event_queue.dequeue()
    }

    fn render_inner(&mut self, area: &Area, writer: &mut impl Writable) {
        let mut flag = self.update;
        // let mut area = area.clone();
        // let mut bitmap = DrawBitmap::new(area.clone());
        for layer_id in self.stack.iter() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            let update_flag = layer.take_update();
            flag = flag || update_flag;
            if flag {
                layer.draw(0, 0, area, &mut self.global);
            }
            // layer.bitmap_draw(0, 0, area, &mut bitmap, &mut self.global);
        }
        self.update = false;
        print_curser(self.mouse_x, self.mouse_y, &mut self.global);
    }

    fn render_window(&mut self, id: usize, writer: &mut impl Writable) {
        let mut area: Option<Area> = None;
        for layer_id in self.stack.iter() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            if *layer_id == id {
                area = Some(layer.area());
            }
            if let Some(ref area) = area {
                layer.draw(0, 0, area, &mut self.global);
            }
        }
        self.update = false;
        print_curser(self.mouse_x, self.mouse_y, &mut self.global);
    }

    fn render_area(&mut self, area: &Area, writer: &mut impl Writable) {
        let mut flag = self.update;
        // let mut area = area.clone();
        // let mut bitmap = DrawBitmap::new(area.clone());
        for layer_id in self.stack.iter() {
            let layer = self.layers.get_mut(layer_id).expect("Not Found");
            let update_flag = layer.take_update();
            flag = flag || update_flag;
            if flag {
                layer.draw(0, 0, area, &mut self.global);
            }
            // layer.bitmap_draw(0, 0, area, &mut bitmap, &mut self.global);
        }
        self.update = false;
        print_curser(self.mouse_x, self.mouse_y, &mut self.global);
    }

    pub fn render(&mut self, area: &Area, writer: &mut impl Writable) {
        self.render_inner(area, writer);
        self.global.draw(0, 0, &self.area(), writer);
    }
}
