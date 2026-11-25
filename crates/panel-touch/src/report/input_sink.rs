use crate::{Contact, Pen};

pub trait InputSink {
    fn contact(&mut self, contact: &Contact);
    fn finish_contacts(&mut self);
    fn pen(&mut self, pen: &Pen, eraser: bool);
    fn release_pen(&mut self);
}
