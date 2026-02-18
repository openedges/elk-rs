use std::cmp::Ordering;

pub trait EventHandler<T> {
    fn handle(&mut self, point: &T);
}

impl<T, F> EventHandler<T> for F
where
    F: FnMut(&T),
{
    fn handle(&mut self, point: &T) {
        self(point);
    }
}

pub struct Scanline;

impl Scanline {
    pub fn execute<T, H, C>(points: impl IntoIterator<Item = T>, comparator: C, handler: &mut H)
    where
        H: EventHandler<T>,
        C: Fn(&T, &T) -> Ordering,
    {
        let mut points: Vec<T> = points.into_iter().collect();
        points.sort_by(|a, b| comparator(a, b));
        for point in &points {
            handler.handle(point);
        }
    }

    pub fn execute_with_handlers<T, C>(
        points: impl IntoIterator<Item = T>,
        comparator: C,
        handlers: &mut [&mut dyn EventHandler<T>],
    ) where
        C: Fn(&T, &T) -> Ordering,
    {
        let mut points: Vec<T> = points.into_iter().collect();
        points.sort_by(|a, b| comparator(a, b));
        for point in &points {
            for handler in handlers.iter_mut() {
                handler.handle(point);
            }
        }
    }
}
