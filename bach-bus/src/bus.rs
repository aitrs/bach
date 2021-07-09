use crate::packet::Packet;
use crate::queue::Queue;
use std::cell::RefCell;

pub type Preempt = dyn FnMut(Packet) + Send + Sync;
pub type Watch = dyn FnMut(Packet) -> bool + Send + Sync;
pub type Out = dyn FnMut() -> Option<Packet> + Send + Sync;

pub struct BusConnection {
    p: Box<Preempt>,
    w: Box<Watch>,
    o: Box<Out>,
}

impl<'z> BusConnection {
    pub fn new<Fp, Fw, Fo>(p: Fp, w: Fw, o: Fo) -> Self
    where
        Fp: 'static + Fn(Packet) + Send + Sync,
        Fw: 'static + FnMut(Packet) -> bool + Send + Sync,
        Fo: 'static + FnMut() -> Option<Packet> + Send + Sync,
    {
        BusConnection {
            p: Box::new(p),
            w: Box::new(w),
            o: Box::new(o),
        }
    }

    pub fn perform(&mut self, p: Option<Packet>) -> Option<Packet> {
        if p.is_some() {
            if (self.w)(p.unwrap()) {
                (self.p)(p.unwrap());
            }
        }

        (self.o)()
    }
}

pub struct Bus {
    cable: Queue<Packet>,
    connections: RefCell<Vec<BusConnection>>,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            cable: Queue::new(),
            connections: RefCell::new(Vec::new()),
        }
    }

    pub fn connect(&self, conn: BusConnection) {
        self.connections.borrow_mut().push(conn);
    }

    pub fn perform(&self) {
        let mut conns = self.connections.borrow_mut();
        let next = self.cable.consume();
        for c in conns.iter_mut() {
            match c.perform(next) {
                Some(out_packet) => {
                    self.cable.push(out_packet);
                }
                None => (),
            }
        }
    }

    pub fn send(&self, p: Packet) {
        println!("Pushing {:?}", p);
        self.cable.push(p);
    }

    pub fn pop(&self) -> Option<Packet> {
        self.cable.consume()
    }

    pub fn con_count(&self) -> usize {
        self.connections.borrow().len()
    }
}

#[cfg(test)]
mod test {
    use crate::bus::*;
    use crate::packet::*;
    #[test]
    fn bus_connection() {
        let mut bs = BusConnection::new(
            |_| {
                assert!(true);
            },
            |p| -> bool {
                match p {
                    Packet::NotifyGood(_) => true,
                    _ => false,
                }
            },
            || -> Option<Packet> { None },
        );
        assert!(bs
            .perform(Some(Packet::new_ng("FOO", "BAR", "BAZ")))
            .is_none());
    }

    #[test]
    fn bus() {
        let b = Bus::new();
        b.connect(BusConnection::new(
            |_| {
                assert!(true);
            },
            |p| -> bool {
                match p {
                    Packet::NotifyGood(_) => true,
                    _ => false,
                }
            },
            || -> Option<Packet> {
                let com = BackupCommand::ChangeHost(None, [192, 168, 1, 1]);
                Some(Packet::BackupCom(PacketCore::from(com)))
            },
        ));
        b.connect(BusConnection::new(
            move |_| {
                assert!(true);
            },
            |p| -> bool {
                match p {
                    Packet::NotifyErr(_) => true,
                    _ => false,
                }
            },
            || -> Option<Packet> { None },
        ));
        b.connect(BusConnection::new(
            move |_| {
                assert!(false);
            },
            |p| -> bool {
                match p {
                    Packet::Terminate => true,
                    _ => false,
                }
            },
            || -> Option<Packet> { None },
        ));
        b.send(Packet::new_ng("FOO", "FAA", "FEE"));
        b.send(Packet::new_ne("BAR", "BOR", "BER"));
        b.send(Packet::new_ne("BAZ", "BOZ", "BEZ"));

        for _ in 0..3 {
            b.perform();
        }

        let end = b.pop();
        b.pop();
        b.pop();
        let empty = b.pop();
        let com = BackupCommand::ChangeHost(None, [192, 168, 1, 1]);
        let endtest = Some(Packet::BackupCom(PacketCore::from(com)));

        assert_eq!(end, endtest);
        assert!(empty.is_none());
    }
}
