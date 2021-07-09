use bach_bus::packet::{core_2_string, PacketCore, CORE_SIZE};

#[macro_export]
macro_rules! str_to_core {
    ($x: ident) => {{
        let ret = [0u8; CORE_SIZE];
        let bytes = $x.as_bytes();
        let len = bytes.len();

        for i in 0..len {
            ret[i] = bytes[i];
        }

        ret
    }};
}

pub enum TcpCommandList {
    Running,
    Loaded,
}

pub enum TcpCommand {
    List(TcpCommandList),
    Status(String),
    Stop(String),
    Terminate,
    Fire(String),
    Undef,
}

impl From<PacketCore> for TcpCommand {
    fn from(item: PacketCore) -> Self {
        let header = core_2_string(&item[0..4]);
        match header.as_str() {
            "LIST" => {
                let subcom = core_2_string(&item[4..CORE_SIZE]);
                match subcom.as_str() {
                    "running" => TcpCommand::List(TcpCommandList::Running),
                    "loaded" => TcpCommand::List(TcpCommandList::Loaded),
                    _ => TcpCommand::Undef,
                }
            }
            "STAT" => {
                let name = core_2_string(&item[4..CORE_SIZE]);
                TcpCommand::Status(name)
            }
            "STOP" => {
                let name = core_2_string(&item[4..CORE_SIZE]);
                TcpCommand::Stop(name)
            }
            "TERM" => TcpCommand::Terminate,
            "FIRE" => {
                let name = core_2_string(&item[4..CORE_SIZE]);
                TcpCommand::Fire(name)
            }
            _ => TcpCommand::Undef,
        }
    }
}
