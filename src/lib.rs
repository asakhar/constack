use core::sync::atomic::{AtomicPtr, Ordering};

struct Node<T> {
    prev: *mut Node<T>,
    value: T,
}

#[derive(Debug)]
pub struct ConStack<T> {
    top: AtomicPtr<Node<T>>,
}

impl<T> Default for ConStack<T> {
    fn default() -> Self {
        Self {
            top: Default::default(),
        }
    }
}

impl<T> ConStack<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push(&self, value: T) {
        let mut node = Box::new(Node {
            prev: core::ptr::null_mut(),
            value,
        });
        let node_ptr = Box::as_mut(&mut node) as *mut _;
        self.top
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
                node.prev = prev;
                Some(node_ptr)
            })
            .unwrap();
        core::mem::forget(node);
    }

    pub fn pop(&self) -> Option<T> {
        let mut ptr = core::ptr::null_mut();
        self.top
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |node| {
                if node == core::ptr::null_mut() {
                    return None;
                }
                ptr = node;
                // Questinable place
                Some(unsafe { (*node).prev })
            })
            .ok()
            .map(|_| unsafe { Box::from_raw(ptr) }.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let stack = ConStack::default();
        stack.push(1);
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn concurrency() {
        let stack = ConStack::default();
        std::thread::scope(|scoped| {
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(i);
                }
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(i);
                }
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(i);
                }
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
        });
        assert_eq!(stack.pop(), None);
    }

    #[test]
    pub fn drop_check() {
        use core::sync::atomic::AtomicUsize;
        static DROP_CNT: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug, PartialEq)]
        struct DC {
            n: i32,
        }

        impl DC {
            fn new(n: i32) -> Self {
                DROP_CNT.fetch_add(1, Ordering::Relaxed);
                Self { n }
            }
        }

        impl Drop for DC {
            fn drop(&mut self) {
                DROP_CNT.fetch_sub(1, Ordering::Relaxed);
            }
        }

        let stack = ConStack::default();
        std::thread::scope(|scoped| {
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(DC::new(i));
                }
                assert!(DROP_CNT.load(Ordering::SeqCst) >= 1000000);
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(DC::new(i));
                }
                assert!(DROP_CNT.load(Ordering::SeqCst) >= 1000000);
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
            scoped.spawn(|| {
                for i in 0..1000000 {
                    stack.push(DC::new(i));
                }
                assert!(DROP_CNT.load(Ordering::SeqCst) >= 1000000);
                std::thread::yield_now();
                for _ in 0..1000000 {
                    stack.pop().unwrap();
                }
            });
        });
        assert_eq!(stack.pop(), None);
        assert_eq!(DROP_CNT.load(Ordering::Relaxed), 0);
    }
}
