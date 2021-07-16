use crate::packet::Packet;
use crate::queue::Queue;
use std::cell::RefCell;

pub type Input = dyn FnMut(Packet) + Send + Sync;
pub type Output = dyn FnMut() -> Option<Packet> + Send + Sync;

pub struct BusConnection {
    i: Box<Input>,
    o: Box<Output>,
}

impl BusConnection {
    pub fn new<Fi, Fo>(i: Fi, o: Fo) -> Self
    where
        Fi: 'static + Fn(Packet) + Send + Sync,
        Fo: 'static + FnMut() -> Option<Packet> + Send + Sync,
    {
        BusConnection {
            i: Box::new(i),
            o: Box::new(o),
        }
    }

    pub fn perform(&mut self, p: Option<Packet>) -> Option<Packet> {
        if let Some(pp) = p {
            (self.i)(pp);
        }

        (self.o)()
    }
}

pub struct Bus {
    cable: Queue<Packet>,
    connections: RefCell<Vec<BusConnection>>,
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
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
            if let Some(out_packet) = c.perform(next) {
                self.cable.push(out_packet);
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
            || -> Option<Packet> {
                let com = BackupCommand::ChangeHost(None, [192, 168, 1, 1]);
                Some(Packet::BackupCom(PacketCore::from(com)))
            },
        ));
        b.connect(BusConnection::new(
            move |_| {
                assert!(true);
            },
            || -> Option<Packet> { None },
        ));
        b.connect(BusConnection::new(
            move |_| {
                assert!(true);
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
