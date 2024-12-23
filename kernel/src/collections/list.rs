use core::{
    clone,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use alloc::boxed::Box;

use super::Length;

struct Node<T> {
    data: T,
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            next: None,
            prev: None,
        }
    }
}

pub struct Curser<T> {
    list: NonNull<List<T>>,
    node: Option<NonNull<Node<T>>>,
}

impl<T> Clone for Curser<T> {
    fn clone(&self) -> Self {
        let Self { list, node } = *self;
        Self { list, node }
    }
}

impl<T> Curser<T> {
    pub fn move_next(&mut self) {
        if let Some(current) = self.node.take() {
            unsafe { self.node = current.as_ref().next };
        }
    }

    pub fn move_prev(&mut self) {
        if let Some(current) = self.node.take() {
            unsafe { self.node = current.as_ref().prev };
        }
    }

    pub fn data(&self) -> Option<&mut T> {
        unsafe {
            self.node
                .map(|mut current| unsafe { &mut current.as_mut().data })
        }
    }

    pub fn remove(&mut self) -> Option<T> {
        self.node.map(|mut current| unsafe {
            let list = self.list.as_mut();
            let mut next_node = current.as_mut().next;
            let mut prev_node = current.as_mut().prev;
            if let Some(mut next) = next_node {
                next.as_mut().prev = prev_node;
            }
            if let Some(mut prev) = prev_node {
                prev.as_mut().next = next_node;
            }
            if let Some(head) = list.head {
                if head == current {
                    list.head = next_node;
                }
            }
            if let Some(tail) = list.tail {
                if tail == current {
                    list.tail = prev_node;
                }
            }
            list.length -= 1;
            let dummy = Box::from_raw(current.as_ptr());
            dummy.data
        })
        // if let Some(mut current) = self.node {
        //     unsafe {
        //         let list = self.list.as_mut();
        //         let mut next_node = current.as_mut().next;
        //         let mut prev_node = current.as_mut().prev;
        //         if let Some(mut next) = next_node {
        //             next.as_mut().prev = prev_node;
        //         }
        //         if let Some(mut prev) = prev_node {
        //             prev.as_mut().next = next_node;
        //         }
        //         if let Some(head) = list.head {
        //             if head == current {
        //                 list.head = next_node;
        //             }
        //         }
        //         if let Some(tail) = list.tail {
        //             if tail == current {
        //                 list.tail = prev_node;
        //             }
        //         }
        //         list.length -= 1;
        //         let dummy = Box::from_raw(current.as_ptr());
        //         return Some(dummy.data);
        //     }
        // }
    }

    pub fn insert_before(&mut self, data: T) {
        let mut boxed_node = Box::new(Node::new(data));
        let mut node = NonNull::new(Box::into_raw(boxed_node));
        if let Some(mut new_node) = node {
            if let Some(mut current_node) = self.node {
                let current = unsafe { current_node.as_mut() };
                let new = unsafe { new_node.as_mut() };
                new.prev = current.prev;
                new.next = Some(current_node);
                current.prev = Some(new_node);
            }
            let list = unsafe { self.list.as_mut() };
            list.length += 1;
            if list.head == self.node {
                list.head = node;
            }
        }
    }

    pub fn insert_next(&mut self, data: T) {
        let mut boxed_node = Box::new(Node::new(data));
        let mut node = NonNull::new(Box::into_raw(boxed_node));
        if let Some(mut new_node) = node {
            if let Some(mut current_node) = self.node {
                let current = unsafe { current_node.as_mut() };
                let new = unsafe { new_node.as_mut() };
                new.prev = Some(current_node);
                new.next = current.next;
                current.next = Some(new_node);
            }
            let list = unsafe { self.list.as_mut() };
            list.length += 1;
            if list.tail == self.node {
                list.tail = node;
            }
        }
    }
}

pub struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    length: usize,
}

impl<T: Clone> List<T> {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            length: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == None
    }

    pub fn front(&mut self) -> Curser<T> {
        Curser {
            list: NonNull::new(self).unwrap(),
            node: self.head.clone(),
        }
        // self.head.map(|node| unsafe { Curser { list: self, node } })
    }

    pub fn tail(&mut self) -> Curser<T> {
        Curser {
            list: NonNull::new(self).unwrap(),
            node: self.tail.clone(),
        }
        // self.tail.map(|node| unsafe {
        //     Curser {
        //         list: NonNull::new(self).unwrap(),
        //         node,
        //     }
        // })
    }

    #[inline]
    pub fn push(&mut self, data: T) {
        self.push_ptr(Box::new(Node::new(data)));
        // self.front().insert_before(data);
    }

    fn push_ptr(&mut self, mut node_box: Box<Node<T>>) {
        let mut node = NonNull::new(Box::into_raw(node_box));
        if let Some(mut node) = node {
            if let Some(mut prev) = self.tail {
                unsafe {
                    prev.as_mut().next = Some(node);
                    node.as_mut().prev = Some(prev);
                }
            }
            self.tail = Some(node);
            if let None = self.head {
                self.head = Some(node);
            }
            self.length += 1;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if let Some(mut node) = self.head {
            unsafe {
                let mut next_node = node.as_ref().next;
                node.as_mut().next = None;
                self.head = next_node;
                if let Some(mut next) = next_node {
                    next.as_mut().prev = None;
                } else {
                    self.tail = None;
                }
                self.length -= 1;
                let dummy = Box::from_raw(node.as_ptr());
                return Some(dummy.data);
            }
        }
        None
    }

    pub fn iter(&mut self) -> CurserIter<T> {
        CurserIter {
            curser: self.front(),
        }
    }
}

impl<T> Length for List<T> {
    fn length(&self) -> usize {
        self.length
    }
}

pub struct CurserIter<T> {
    curser: Curser<T>,
}

impl<T> Iterator for CurserIter<T> {
    type Item = Curser<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let before = self.curser.clone();
        self.curser.move_next();
        Some(before)
    }
}

pub trait RawNode {
    fn prev(&self) -> Option<&mut Self>;
    fn next(&self) -> Option<&mut Self>;
    fn set_prev(&mut self, node: Option<&mut Self>);
    fn set_next(&mut self, node: Option<&mut Self>);

    fn insert_prev(&mut self, node: &mut Self) {
        node.set_prev(self.prev());
        node.set_next(Some(self));
        self.set_prev(Some(node));
    }

    fn insert_next(&mut self, node: &mut Self) {
        node.set_next(self.next());
        node.set_prev(Some(self));
        self.set_next(Some(node));
    }

    fn remove(&mut self) {
        if let Some(prev) = self.prev() {
            prev.set_next(self.next());
        }
        if let Some(next) = self.next() {
            next.set_prev(self.prev());
        }
        self.set_prev(None);
        self.set_next(None);
    }
}

pub struct RawList<T: RawNode> {
    head: Option<NonNull<T>>,
    tail: Option<NonNull<T>>,
    lenght: usize,
}

impl<T: RawNode> RawList<T> {
    pub const fn empty() -> Self {
        Self {
            head: None,
            tail: None,
            lenght: 0,
        }
    }

    pub fn push(&mut self, data: &mut T) {
        self.lenght += 1;
        let head = self.head;
        self.head = NonNull::new(data);

        if let Some(mut head_ptr) = head {
            let head = unsafe { head_ptr.as_mut() };
            data.insert_next(head);
        } else {
            self.tail = self.head;
        }
    }

    pub fn pop(&mut self) -> Option<&'static mut T> {
        if let Some(mut tail_ptr) = self.tail {
            let tail = unsafe { tail_ptr.as_mut() };
            self.tail = tail.prev().map(|task| NonNull::new(task).unwrap());
            tail.remove();
            self.lenght -= 1;

            if self.tail.is_none() {
                self.head = None;
            }
            Some(tail)
        } else {
            None
        }
    }

    pub const fn len(&self) -> usize {
        self.lenght
    }

    pub fn iter(&self) -> RawListIter<T> {
        RawListIter(self.head)
    }
}

pub struct RawListIter<T: RawNode>(Option<NonNull<T>>);

impl<T: RawNode + 'static> Iterator for RawListIter<T> {
    type Item = &'static mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut data_ptr) = self.0 {
            let data = unsafe { data_ptr.as_mut() };
            self.0 = data.next().map(|next| NonNull::new(next).unwrap());
            Some(data)
        } else {
            None
        }
    }
}
