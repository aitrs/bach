use std::cell::RefCell;

#[derive(Clone, Debug)]
enum InnerQueue<T: Copy> {
    Node(T, Box<InnerQueue<T>>),
    Null,
}

impl<T: Copy> InnerQueue<T> {
    pub fn new() -> Self {
        InnerQueue::Null
    }

    pub fn from_item(i: T) -> Self {
        InnerQueue::Node(i, Box::new(InnerQueue::Null))
    }

    pub fn is_null(&self) -> bool {
        match self {
            InnerQueue::Node(_, _) => false,
            InnerQueue::Null => true,
        }
    }

    pub fn set_null(&mut self) {
        *self = InnerQueue::Null;
    }

    pub fn find<F>(&mut self, selector: F) -> Option<T>
    where
        F: 'static + FnMut(&T) -> bool,
    {
        fn find_r<T: Copy, F>(q: &mut InnerQueue<T>, mut selector: F) -> Option<T>
        where
            F: 'static + FnMut(&T) -> bool,
        {
            match q {
                InnerQueue::Node(ref item, ref mut next) => {
                    if selector(item) {
                        Some(item.clone())
                    } else {
                        find_r(next, selector)
                    }
                }
                InnerQueue::Null => None,
            }
        }

        find_r(self, selector)
    }

    pub fn push_ext(ext: Self, i: T) -> Self {
        InnerQueue::Node(i, Box::new(ext))
    }

    pub fn pop(&mut self) -> Option<T> {
        match *self {
            InnerQueue::Node(ref it, ref mut next) => {
                if next.is_null() {
                    let copy = it.clone();
                    drop(it);
                    *self = InnerQueue::Null;
                    return Some(copy);
                } else {
                    next.pop()
                }
            }
            InnerQueue::Null => None,
        }
    }

    pub fn watch(&self) -> Option<T> {
        match *self {
            InnerQueue::Node(ref it, ref next) => {
                if next.is_null() {
                    return Some(it.clone());
                } else {
                    next.watch()
                }
            }
            InnerQueue::Null => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Queue<T: 'static + Copy> {
    inner: RefCell<InnerQueue<T>>,
}

impl<T: Copy> Queue<T> {
    pub fn new() -> Self {
        Queue {
            inner: RefCell::new(InnerQueue::new()),
        }
    }

    pub fn from_item(i: T) -> Self {
        Queue {
            inner: RefCell::new(InnerQueue::from_item(i)),
        }
    }

    pub fn empty(&self) -> bool {
        self.inner.borrow().is_null()
    }

    pub fn clear(&self) {
        self.inner.borrow_mut().set_null();
    }

    pub fn find<F>(&self, selector: F) -> Option<T>
    where
        F: 'static + FnMut(&T) -> bool,
    {
        self.inner.borrow_mut().find(selector)
    }

    pub fn push(&self, i: T) -> &Self {
        let q = self.clone().inner.into_inner();
        self.inner.replace(InnerQueue::push_ext(q, i));
        self
    }

    pub fn consume(&self) -> Option<T> {
        let mut i = self.inner.borrow_mut();
        i.pop()
    }

    pub fn watch(&self) -> Option<T> {
        let val = self.inner.borrow().watch();
        val
    }
}

impl<T: Copy> Default for Queue<T> {
    fn default() -> Self {
        Queue::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::Queue;
    #[test]
    fn queue_new() {
        let q: Queue<u32> = Queue::new();
        assert!(q.empty());
    }

    #[test]
    fn queue_from_item() {
        let q: Queue<u32> = Queue::from_item(5);
        assert!(!q.empty());
        let elt = q.consume();
        assert!(elt.is_some());
        assert_eq!(elt.unwrap(), 5);
    }

    #[test]
    fn queue_is_null() {
        let q: Queue<u32> = Queue::new();
        assert!(q.empty());
    }

    #[test]
    fn queue_clear() {
        let q: Queue<u32> = Queue::new();
        q.push(1);
        q.push(2);
        q.push(3);
        q.clear();
        assert!(q.empty());
    }

    #[test]
    fn queue_find() {
        let q: Queue<u32> = Queue::new();
        q.push(5);
        q.push(8);
        q.push(9);
        let n = q.find(|x| *x == 8);
        assert_eq!(n.unwrap(), 8);
    }

    #[test]
    fn queue_push() {
        let q: Queue<u32> = Queue::new();
        q.push(5);
        assert!(!q.empty());
    }

    #[test]
    fn queue_consume() {
        let q: Queue<u32> = Queue::new();
        for i in 0..4 {
            q.push(i);
        }
        for i in 0..4 {
            let n = q.consume();
            assert_eq!(n.unwrap(), i);
        }
        let nbis = q.consume();
        assert!(nbis.is_none());

        let q2: Queue<u32> = Queue::new();
        let n2 = q2.consume();
        assert!(n2.is_none());
    }

    #[test]
    fn queue_watch() {
        let q: Queue<u32> = Queue::new();
        for i in 0..4 {
            q.push(i);
        }
        let n = q.watch();
        let n2 = q.watch();
        assert_eq!(n.unwrap(), 0);
        assert_eq!(n2.unwrap(), 0);
        q.consume();
        let n = q.watch();
        assert_eq!(n.unwrap(), 1);
        let q2: Queue<u32> = Queue::new();
        let n3 = q2.watch();
        assert!(n3.is_none());
    }
}
