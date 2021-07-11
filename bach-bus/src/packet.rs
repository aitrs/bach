use std::path::PathBuf;

pub const CORE_SIZE: usize = 1024;
pub const NAME_SIZE: usize = 100;

macro_rules! mk_core_str {
    ($x: ident) => {{
        let mlen = $x.len();
        let mut pcore = [0u8; CORE_SIZE];

        if mlen < CORE_SIZE {
            let bytes = $x.as_bytes();
            let blen = bytes.len();

            for i in 0..blen {
                pcore[i] = bytes[i];
            }
        } else {
            let bytes = $x.as_bytes();
            for i in 0..CORE_SIZE {
                pcore[i] = bytes[i];
            }
        }

        pcore
    }};
}

macro_rules! mk_core_extended {
    ($message: ident, $provider: ident, $stage: ident) => {{
        let itlen = CORE_SIZE / 3;
        let mut pcore = [0u8; CORE_SIZE];
        let mut curdex = 0;
        let mut trim = move |st: &str, c: &mut [u8]| {
            let bytes = st.as_bytes();
            let totlen = if bytes.len() > itlen {
                itlen
            } else {
                bytes.len()
            };
            for i in 0..totlen {
                c[i + curdex] = bytes[i];
            }
            curdex += itlen;
        };

        trim($message, &mut pcore);
        trim($provider, &mut pcore);
        trim($stage, &mut pcore);
        pcore
    }};
}

macro_rules! mk_core_bytes {
    ($x: ident) => {{
        let mlen = $x.len();
        let mut pcore = [0u8; CORE_SIZE];

        if mlen < CORE_SIZE {
            for i in 0..mlen {
                pcore[i] = $x[i];
            }
        } else {
            for i in 0..CORE_SIZE {
                pcore[i] = $x[i];
            }
        }

        pcore
    }};
}

pub fn core_2_string(s: &[u8]) -> String {
    let mut ret = String::new();
    for c in s {
        if *c != 0u8 {
            ret.push(*c as char);
        }
    }
    ret
}

#[derive(Debug, Clone)]
pub struct PacketError(String);

impl PacketError {
    pub fn new(m: &str) -> Self {
        PacketError(m.to_string())
    }
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Packet Error : {}", self.0))
    }
}

pub type PacketResult<T> = Result<T, PacketError>;

pub type PacketCore = [u8; CORE_SIZE];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotifyCommand {
    ShutUp(Option<String>),
    Error(Option<String>),
    Warning(Option<String>),
    Debug(Option<String>),
    Undef,
}

impl From<PacketCore> for NotifyCommand {
    fn from(item: PacketCore) -> Self {
        let core_header = match String::from_utf8(item[0..4].to_vec()) {
            Ok(s) => s,
            Err(_) => return NotifyCommand::Undef,
        };

        let core_name = core_2_string(&item[4..CORE_SIZE]);
        let name = if !core_name.is_empty() {
            Some(core_name)
        } else {
            None
        };

        match core_header.as_str() {
            "SHUT" => NotifyCommand::ShutUp(name),
            "ERRO" => NotifyCommand::Error(name),
            "WARN" => NotifyCommand::Warning(name),
            "DEBU" => NotifyCommand::Debug(name),
            _ => NotifyCommand::Undef,
        }
    }
}

impl From<NotifyCommand> for PacketCore {
    fn from(item: NotifyCommand) -> Self {
        let str2ret = |s: &[u8], opt: Option<String>| {
            let mut ret = [0u8; CORE_SIZE];
            ret[..4].clone_from_slice(&s[..4]);

            let n = match &opt {
                Some(st) => st.as_bytes(),
                None => "".as_bytes(),
            };

            let len = if n.len() < NAME_SIZE {
                n.len()
            } else {
                NAME_SIZE
            };
            ret[4..len+4].clone_from_slice(&n[..len]);

            ret
        };
        match item {
            NotifyCommand::ShutUp(d) => str2ret(b"SHUT", d),
            NotifyCommand::Error(d) => str2ret(b"ERRO", d),
            NotifyCommand::Warning(d) => str2ret(b"WARN", d),
            NotifyCommand::Debug(d) => str2ret(b"DEBU", d),
            _ => str2ret(b"WARN", Some("".to_string())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatchCommand {
    ChangeTarget(Option<String>, String),
    TestTarget(Option<String>, String),
    PrintTarget(Option<String>),
    TryRepair(Option<String>, String),
    Undef,
}

impl From<PacketCore> for WatchCommand {
    fn from(item: PacketCore) -> Self {
        let core_header = match String::from_utf8(item[0..4].to_vec()) {
            Ok(s) => s,
            Err(_) => return WatchCommand::Undef,
        };

        let core_name = core_2_string(&item[4..NAME_SIZE]);
        let name = if !core_name.is_empty() {
            Some(core_name)
        } else {
            None
        };
        let ressource = core_2_string(&item[NAME_SIZE + 4..CORE_SIZE]);

        match core_header.as_str() {
            "CHTA" => WatchCommand::ChangeTarget(name, ressource),
            "TSTA" => WatchCommand::TestTarget(name, ressource),
            "PRTA" => WatchCommand::PrintTarget(name),
            "TRRP" => WatchCommand::TryRepair(name, ressource),
            _ => WatchCommand::Undef,
        }
    }
}

impl From<WatchCommand> for PacketCore {
    fn from(item: WatchCommand) -> Self {
        let str2ret = |pre: &[u8], opt: Option<String>, s: &str| {
            let mut ret = [0u8; CORE_SIZE];
            ret[..4].clone_from_slice(&pre[..4]);
            let bytes = s.as_bytes();

            let n = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            let nlen = if n.len() < NAME_SIZE {
                n.len()
            } else {
                NAME_SIZE
            };
            ret[4..nlen+4].clone_from_slice(&n[..nlen]);

            let len = if bytes.len() < CORE_SIZE - NAME_SIZE - 4 {
                bytes.len()
            } else {
                CORE_SIZE - NAME_SIZE - 4
            };
            for i in 0..len {
                ret[i + NAME_SIZE + 4] = bytes[i];
            }

            ret
        };
        match item {
            WatchCommand::ChangeTarget(n, r) => str2ret(b"CHTA", n, &r),
            WatchCommand::TestTarget(n, r) => str2ret(b"TSTA", n, &r),
            WatchCommand::PrintTarget(n) => str2ret(b"PRTA", n, ""),
            WatchCommand::TryRepair(n, r) => str2ret(b"TRRP", n, &r),
            _ => str2ret(b"PRTA", None, ""),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCredentials(String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackupCommand {
    Fire(Option<String>),
    ChangeTarget(Option<String>, PathBuf),
    ChangeSource(Option<String>, PathBuf),
    HasHostCapability(Option<String>),
    ChangeHost(Option<String>, [u8; 4]),
    ChangeHostCredentials(Option<String>, HostCredentials),
    PingHost(Option<String>),
    Print(Option<String>),
    Undef,
}

impl From<PacketCore> for BackupCommand {
    fn from(item: PacketCore) -> Self {
        let core_header = match String::from_utf8(item[0..4].to_vec()) {
            Ok(s) => s,
            Err(_) => return BackupCommand::Undef,
        };

        let core_name = core_2_string(&item[4..NAME_SIZE]);
        let name = if !core_name.is_empty() {
            Some(core_name)
        } else {
            None
        };

        let sl2ar = |sl: &[u8]| {
            let mut ret = [0u8; 4];
            ret[..4].clone_from_slice(&sl[..4]);

            ret
        };

        match core_header.as_ref() {
            "FIRE" => BackupCommand::Fire(name),
            "CHTA" => BackupCommand::ChangeTarget(
                name,
                PathBuf::from(core_2_string(&item[NAME_SIZE + 4..CORE_SIZE])),
            ),
            "CHSR" => BackupCommand::ChangeSource(
                name,
                PathBuf::from(core_2_string(&item[NAME_SIZE + 4..CORE_SIZE])),
            ),
            "HAHO" => BackupCommand::HasHostCapability(name),
            "CHHO" => BackupCommand::ChangeHost(name, sl2ar(&item[NAME_SIZE + 4..NAME_SIZE + 8])),
            "CHHC" => BackupCommand::ChangeHostCredentials(
                name,
                HostCredentials(
                    core_2_string(&item[NAME_SIZE + 4..(CORE_SIZE / 2) - (NAME_SIZE / 2) - 2]),
                    core_2_string(
                        &item[(CORE_SIZE / 2) - (NAME_SIZE) / 2 + 2 + NAME_SIZE..CORE_SIZE],
                    ),
                ),
            ),
            "PIHO" => BackupCommand::PingHost(name),
            "PRNT" => BackupCommand::Print(name),
            _ => BackupCommand::Undef,
        }
    }
}

impl From<BackupCommand> for PacketCore {
    fn from(item: BackupCommand) -> Self {
        let write_header = |s: &str, opt: Option<String>| {
            let mut ret = [0u8; CORE_SIZE];
            let bytes = s.as_bytes();
            let n = match &opt {
                Some(s) => s.as_bytes(),
                None => b"",
            };

            let nlen = if n.len() < NAME_SIZE {
                n.len()
            } else {
                NAME_SIZE
            };
            
            ret[..4].clone_from_slice(&bytes[..4]);
            ret[4..nlen+4].clone_from_slice(&bytes[..nlen]);

            ret
        };

        let retpaths = |head: &str, opt: Option<String>, p: PathBuf| {
            let bytes = p.as_path().to_str().unwrap_or("").as_bytes();
            let wlen = if bytes.len() < CORE_SIZE - 4 {
                bytes.len()
            } else {
                CORE_SIZE - 4
            };
            let mut ret = write_header(head, opt);
            for i in 0..wlen {
                ret[i + 4 + NAME_SIZE] = bytes[i];
            }

            ret
        };

        let ip = |head: &str, opt: Option<String>, ip: [u8; 4]| {
            let mut ret = write_header(head, opt);
            for i in 0..4 {
                ret[i + 4 + NAME_SIZE] = ip[i];
            }

            ret
        };

        match item {
            BackupCommand::Fire(n) => write_header("FIRE", n),
            BackupCommand::ChangeTarget(n, p) => retpaths("CHTA", n, p),
            BackupCommand::ChangeSource(n, p) => retpaths("CHSR", n, p),
            BackupCommand::HasHostCapability(n) => write_header("HAHO", n),
            BackupCommand::ChangeHost(n, a) => ip("CHHO", n, a),
            BackupCommand::ChangeHostCredentials(n, creds) => {
                let ubytes = creds.0.as_bytes();
                let pbytes = creds.1.as_bytes();
                let maxlen = CORE_SIZE / 2 - NAME_SIZE / 2 - 2;
                let start1 = NAME_SIZE + 4;
                let start2 = CORE_SIZE / 2 - NAME_SIZE / 2 + 2 + NAME_SIZE;
                let ulen = if ubytes.len() < maxlen {
                    ubytes.len()
                } else {
                    maxlen
                };
                let plen = if pbytes.len() < maxlen {
                    pbytes.len()
                } else {
                    maxlen
                };
                let mut ret = write_header("CHHC", n);
                ret[start1..ulen+start1].clone_from_slice(&ubytes[..ulen]);
                ret[start2..plen+start2].clone_from_slice(&pbytes[..plen]);

                ret
            }
            BackupCommand::PingHost(n) => write_header("PIHO", n),
            BackupCommand::Print(n) => write_header("PRNT", n),
            _ => write_header("PRNT", None),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoggerCommand {
    Write(String),
    Undef,
}

impl From<PacketCore> for LoggerCommand {
    fn from(item: PacketCore) -> Self {
        let core_header = match String::from_utf8(item[0..4].to_vec()) {
            Ok(s) => s,
            Err(_) => return LoggerCommand::Undef,
        };

        match core_header.as_str() {
            "WRIT" => LoggerCommand::Write(core_2_string(&item[4..CORE_SIZE])),
            _ => LoggerCommand::Undef,
        }
    }
}

impl From<LoggerCommand> for PacketCore {
    fn from(item: LoggerCommand) -> Self {
        let mut ret = [0u8; CORE_SIZE];
        match item {
            LoggerCommand::Write(s) => {
                let b = s.as_bytes();
                let h = b"WRIT";
                ret[..4].clone_from_slice(&h[..4]);
                
                let len = if b.len() < CORE_SIZE - 4 {
                    b.len()
                } else {
                    CORE_SIZE - 4
                };
                ret[4..len+4].clone_from_slice(&b[..len]);
            }
            _ => {
                let h = b"WRIT";
                ret[..4].clone_from_slice(&h[..4]);
            }
        }

        ret
    }
}

pub struct Notification {
    pub message: String,
    pub provider: String,
    pub stage: String,
    pub good: bool,
}

impl Notification {
    pub fn to_string(&self) -> String {
        if self.good {
            format!("{}:{} at stage {}", self.provider, self.message, self.stage)
        } else {
            "Wrong Notification Format".to_string()
        }
    }
}

impl From<Packet> for Notification {
    fn from(item: Packet) -> Self {
        let untrim = move |core: &[u8]| -> Self {
            let itlen = CORE_SIZE / 3;
            let mut message = String::new();
            let mut provider = String::new();
            let mut stage = String::new();
            let mut curdex = 0;
            for i in 0..3 {
                for j in 0..itlen {
                    if core[j + curdex] == 0u8 {
                        break;
                    }
                    if i == 0 {
                        message.push(core[j + curdex] as char)
                    } else if i == 1 {
                        provider.push(core[j + curdex] as char)
                    } else if i == 2 {
                        stage.push(core[j + curdex] as char)
                    }
                }
                curdex += itlen;
            }
            Notification {
                message,
                stage,
                provider,
                good: true,
            }
        };

        match item {
            Packet::NotifyGood(core) => untrim(&core),
            Packet::NotifyWarn(core) => untrim(&core),
            Packet::NotifyErr(core) => untrim(&core),
            _ => Notification {
                message: String::new(),
                stage: String::new(),
                provider: String::new(),
                good: false,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    NotifyGood(PacketCore),
    NotifyWarn(PacketCore),
    NotifyErr(PacketCore),
    NotifyCom(PacketCore),
    WatchReportGood(PacketCore),
    WatchReportWarn(PacketCore),
    WatchReportFail(PacketCore),
    WatchHold,
    WatchCom(PacketCore),
    BackupCom(PacketCore),
    LoggerCom(PacketCore),
    Stop(PacketCore),
    Alive(PacketCore),
    Terminate,
}

impl Packet {
    pub fn new_ng(message: &str, provider: &str, stage: &str) -> Self {
        let core = mk_core_extended!(message, provider, stage);
        Packet::NotifyGood(core)
    }

    pub fn new_nw(message: &str, provider: &str, stage: &str) -> Self {
        let core = mk_core_extended!(message, provider, stage);
        Packet::NotifyWarn(core)
    }

    pub fn new_ne(message: &str, provider: &str, stage: &str) -> Self {
        let core = mk_core_extended!(message, provider, stage);
        Packet::NotifyErr(core)
    }

    pub fn new_nc(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::NotifyCom(core)
    }

    pub fn new_wrg() -> Self {
        let core = [0u8; CORE_SIZE];
        Packet::WatchReportGood(core)
    }

    pub fn new_wrw(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::WatchReportWarn(core)
    }

    pub fn new_wrf(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::WatchReportFail(core)
    }

    pub fn new_wh() -> Self {
        Packet::WatchHold
    }

    pub fn new_wc(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::WatchCom(core)
    }

    pub fn new_bc(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::BackupCom(core)
    }

    pub fn new_lc(data: &[u8]) -> Self {
        let core = mk_core_bytes!(data);
        Packet::LoggerCom(core)
    }

    pub fn new_stop(name: &str) -> Self {
        let core = mk_core_str!(name);
        Packet::Stop(core)
    }

    pub fn new_term() -> Self {
        Packet::Terminate
    }

    pub fn new_alive(name: &str) -> Self {
        let header = b"ALIVE";
        let bytes = name.as_bytes();
        let headlen = header.len();
        let blen = if bytes.len() + headlen > CORE_SIZE {
            CORE_SIZE - headlen
        } else {
            bytes.len()
        };
        let mut core = [0u8; CORE_SIZE];
        for i in 0..headlen {
            core[i] = header[i];
        }

        for i in 0..blen {
            core[i + headlen] = bytes[i];
        }

        Packet::Alive(core)
    }

    pub fn get_core(&self) -> PacketCore {
        match self {
            Packet::NotifyGood(e) => *e,
            Packet::NotifyWarn(e) => *e,
            Packet::NotifyErr(e) => *e,
            Packet::NotifyCom(e) => *e,
            Packet::WatchReportGood(e) => *e,
            Packet::WatchReportWarn(e) => *e,
            Packet::WatchReportFail(e) => *e,
            Packet::WatchHold => [0u8; CORE_SIZE],
            Packet::WatchCom(e) => *e,
            Packet::BackupCom(e) => *e,
            Packet::LoggerCom(e) => *e,
            Packet::Stop(e) => *e,
            Packet::Terminate => [0u8; CORE_SIZE],
            Packet::Alive(e) => *e,
        }
    }
}

pub fn parse_alive(packet: Packet) -> PacketResult<String> {
    match packet {
        Packet::Alive(core) => {
            let name = core_2_string(&core["ALIVE".len()..CORE_SIZE]);
            Ok(name)
        }
        _ => Err(PacketError::new("Not an ALIVE packet")),
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::*;

    #[test]
    fn packet_core_to_nofify_command() {
        let mut core1 = [0u8; CORE_SIZE];
        let mut core2 = [0u8; CORE_SIZE];
        let mut core3 = [0u8; CORE_SIZE];
        let mut core4 = [0u8; CORE_SIZE];
        let mut core5 = [0u8; CORE_SIZE];
        let head1 = b"SHUT";
        let head2 = b"ERRO";
        let head3 = b"WARN";
        let head4 = b"DEBU";
        let head5 = b"SHUU";
        let name = b"Dummy";
        let hlen = head1.len();
        for i in 0..hlen {
            core1[i] = head1[i];
            core2[i] = head2[i];
            core3[i] = head3[i];
            core4[i] = head4[i];
            core5[i] = head5[i];
        }
        for i in 0..name.len() {
            core1[i + 4] = name[i];
            core3[i + 4] = name[i];
            core5[i + 4] = name[i];
        }

        let nc1 = NotifyCommand::from(core1);
        let nc2 = NotifyCommand::from(core2);
        let nc3 = NotifyCommand::from(core3);
        let nc4 = NotifyCommand::from(core4);
        let nc5 = NotifyCommand::from(core5);

        assert_eq!(nc1, NotifyCommand::ShutUp(Some("Dummy".to_string())));
        assert_eq!(nc2, NotifyCommand::Error(None));
        assert_eq!(nc3, NotifyCommand::Warning(Some("Dummy".to_string())));
        assert_eq!(nc4, NotifyCommand::Debug(None));
        assert_eq!(nc5, NotifyCommand::Undef);
    }

    #[test]
    fn notify_command_to_packet_core() {
        let b1 = b"SHUT";
        let b2 = b"ERRO";
        let b3 = b"WARN";
        let b4 = b"DEBU";
        let t1 = PacketCore::from(NotifyCommand::ShutUp(Some("Dummy".to_string())));
        let t2 = PacketCore::from(NotifyCommand::Error(None));
        let t3 = PacketCore::from(NotifyCommand::Warning(Some("Dummy".to_string())));
        let t4 = PacketCore::from(NotifyCommand::Debug(None));
        let t5 = PacketCore::from(NotifyCommand::Undef);

        for i in 0..4 {
            assert_eq!(b1[i], t1[i]);
            assert_eq!(b2[i], t2[i]);
            assert_eq!(b3[i], t3[i]);
            assert_eq!(b4[i], t4[i]);
            assert_eq!(b3[i], t5[i]);
        }

        for i in 4..CORE_SIZE {
            assert_eq!(t2[i], 0u8);
            assert_eq!(t4[i], 0u8);
            assert_eq!(t5[i], 0u8);
        }

        let bytes = b"Dummy";

        for i in 0..bytes.len() {
            assert_eq!(t1[i + 4], bytes[i]);
            assert_eq!(t3[i + 4], bytes[i]);
        }
    }

    #[test]
    fn packet_core_to_watch_command() {
        let genbuf = |h: &[u8], n: &[u8], r: &[u8]| -> PacketCore {
            let mut buf = [0u8; CORE_SIZE];
            for i in 0..4 {
                buf[i] = h[i];
            }

            for i in 0..n.len() {
                buf[i + 4] = n[i];
            }

            let len = r.len();

            for i in 0..len {
                buf[i + 4 + NAME_SIZE] = r[i];
            }

            buf
        };
        let w1 = WatchCommand::from(genbuf(b"CHTA", b"Dummy", b"FOO"));
        let w2 = WatchCommand::from(genbuf(b"TSTA", b"Dummy", b"BAR"));
        let w3 = WatchCommand::from(genbuf(b"PRTA", b"Dummy", b""));
        let w4 = WatchCommand::from(genbuf(b"TRRP", b"Dummy", b"BAZ"));
        let w5 = WatchCommand::from(genbuf(b"BWAA", b"Dummy", b"HOHO"));

        assert_eq!(
            w1,
            WatchCommand::ChangeTarget(Some("Dummy".to_string()), "FOO".to_string())
        );
        assert_eq!(
            w2,
            WatchCommand::TestTarget(Some("Dummy".to_string()), "BAR".to_string())
        );
        assert_eq!(w3, WatchCommand::PrintTarget(Some("Dummy".to_string())));
        assert_eq!(
            w4,
            WatchCommand::TryRepair(Some("Dummy".to_string()), "BAZ".to_string())
        );
        assert_eq!(w5, WatchCommand::Undef);
    }

    #[test]
    fn watch_command_to_packet_core() {
        let genbuf = |h: &[u8], n: &[u8], r: &[u8]| -> PacketCore {
            let mut buf = [0u8; CORE_SIZE];
            for i in 0..4 {
                buf[i] = h[i];
            }

            for i in 0..n.len() {
                buf[i + 4] = n[i];
            }

            let len = r.len();

            for i in 0..len {
                buf[i + 4 + NAME_SIZE] = r[i];
            }

            buf
        };
        let p1 = PacketCore::from(WatchCommand::ChangeTarget(
            Some("Dummy".to_string()),
            "FOO".to_string(),
        ));
        let p2 = PacketCore::from(WatchCommand::TestTarget(
            Some("Dummy".to_string()),
            "BAR".to_string(),
        ));
        let p3 = PacketCore::from(WatchCommand::PrintTarget(Some("Dummy".to_string())));
        let p4 = PacketCore::from(WatchCommand::TryRepair(
            Some("Dummy".to_string()),
            "BAZ".to_string(),
        ));
        let p5 = PacketCore::from(WatchCommand::Undef);
        let b1 = genbuf(b"CHTA", b"Dummy", b"FOO");
        let b2 = genbuf(b"TSTA", b"Dummy", b"BAR");
        let b3 = genbuf(b"PRTA", b"Dummy", b"");
        let b4 = genbuf(b"TRRP", b"Dummy", b"BAZ");
        let b5 = genbuf(b"PRTA", b"", b"");

        for i in 0..CORE_SIZE {
            assert_eq!(p1[i], b1[i]);
            assert_eq!(p2[i], b2[i]);
            assert_eq!(p3[i], b3[i]);
            assert_eq!(p4[i], b4[i]);
            assert_eq!(p5[i], b5[i]);
        }
    }

    #[test]
    fn packet_core_to_backup_command() {
        let genfire = |opt: Option<String>| -> PacketCore {
            let h = b"FIRE";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }

            let bytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..bytes.len() {
                ret[i + 4] = bytes[i];
            }

            ret
        };

        let frompbuf = |h: &[u8], opt: Option<String>, p: PathBuf| -> PacketCore {
            let pbytes = p.as_path().to_str().unwrap().as_bytes();
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            let len = pbytes.len();
            for i in 0..len {
                ret[i + 4 + NAME_SIZE] = pbytes[i];
            }

            ret
        };

        let gencap = |opt: Option<String>| -> PacketCore {
            let mut ret = [0u8; CORE_SIZE];
            let h = b"HAHO";
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let genchhost = |ip: &[u8], opt: Option<String>| -> PacketCore {
            let h = b"CHHO";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
                ret[i + 4 + NAME_SIZE] = ip[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let gencreds = |u: &str, p: &str, opt: Option<String>| -> PacketCore {
            let h = b"CHHC";
            let mut ret = [0u8; CORE_SIZE];
            let bu = u.as_bytes();
            let bp = p.as_bytes();
            let ulen = bu.len();
            let plen = bp.len();
            let start1 = NAME_SIZE + 4;
            let start2 = CORE_SIZE / 2 - NAME_SIZE / 2 + 2 + NAME_SIZE;

            for i in 0..4 {
                ret[i] = h[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            for i in 0..ulen {
                ret[i + start1] = bu[i];
            }

            for i in 0..plen {
                ret[i + start2] = bp[i];
            }

            ret
        };

        let genping = |opt: Option<String>| -> PacketCore {
            let h = b"PIHO";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let genprint = |opt: Option<String>| -> PacketCore {
            let h = b"PRNT";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let genmeh = || -> PacketCore {
            let h = b"BWAA";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }

            ret
        };

        let b1 = BackupCommand::from(genfire(Some("Dummy".to_string())));
        let b2 = BackupCommand::from(frompbuf(
            b"CHTA",
            None,
            PathBuf::from("/foo/bar".to_string()),
        ));
        let b3 = BackupCommand::from(frompbuf(
            b"CHSR",
            Some("Dummy".to_string()),
            PathBuf::from("/foo/bar/baz".to_string()),
        ));
        let b4 = BackupCommand::from(gencap(None));
        let b5 = BackupCommand::from(genchhost(&[192, 168, 1, 1], Some("Dummy".to_string())));
        let b6 = BackupCommand::from(gencreds("foo", "bar", None));
        let b7 = BackupCommand::from(genping(Some("Dummy".to_string())));
        let b8 = BackupCommand::from(genprint(None));
        let b9 = BackupCommand::from(genmeh());

        assert_eq!(b1, BackupCommand::Fire(Some("Dummy".to_string())));
        assert_eq!(
            b2,
            BackupCommand::ChangeTarget(None, PathBuf::from("/foo/bar"))
        );
        assert_eq!(
            b3,
            BackupCommand::ChangeSource(Some("Dummy".to_string()), PathBuf::from("/foo/bar/baz"))
        );
        assert_eq!(b4, BackupCommand::HasHostCapability(None));
        assert_eq!(
            b5,
            BackupCommand::ChangeHost(Some("Dummy".to_string()), [192, 168, 1, 1])
        );
        assert_eq!(
            b6,
            BackupCommand::ChangeHostCredentials(
                None,
                HostCredentials("foo".to_string(), "bar".to_string())
            )
        );
        assert_eq!(b7, BackupCommand::PingHost(Some("Dummy".to_string())));
        assert_eq!(b8, BackupCommand::Print(None));
        assert_eq!(b9, BackupCommand::Undef);
    }

    #[test]
    fn backup_command_to_packet_core() {
        let genfire = |opt: Option<String>| -> PacketCore {
            let h = b"FIRE";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }

            let bytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..bytes.len() {
                ret[i + 4] = bytes[i];
            }

            ret
        };

        let frompbuf = |h: &[u8], opt: Option<String>, p: PathBuf| -> PacketCore {
            let pbytes = p.as_path().to_str().unwrap().as_bytes();
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            let len = pbytes.len();
            for i in 0..len {
                ret[i + 4 + NAME_SIZE] = pbytes[i];
            }

            ret
        };

        let gencap = |opt: Option<String>| -> PacketCore {
            let mut ret = [0u8; CORE_SIZE];
            let h = b"HAHO";
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let genchhost = |ip: &[u8], opt: Option<String>| -> PacketCore {
            let h = b"CHHO";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
                ret[i + 4 + NAME_SIZE] = ip[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let gencreds = |u: &str, p: &str, opt: Option<String>| -> PacketCore {
            let h = b"CHHC";
            let mut ret = [0u8; CORE_SIZE];
            let bu = u.as_bytes();
            let bp = p.as_bytes();
            let ulen = bu.len();
            let plen = bp.len();
            let start1 = NAME_SIZE + 4;
            let start2 = CORE_SIZE / 2 - NAME_SIZE / 2 + 2 + NAME_SIZE;

            for i in 0..4 {
                ret[i] = h[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };

            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            for i in 0..ulen {
                ret[i + start1] = bu[i];
            }

            for i in 0..plen {
                ret[i + start2] = bp[i];
            }

            ret
        };

        let genping = |opt: Option<String>| -> PacketCore {
            let h = b"PIHO";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }

            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let genprint = |opt: Option<String>| -> PacketCore {
            let h = b"PRNT";
            let mut ret = [0u8; CORE_SIZE];
            for i in 0..4 {
                ret[i] = h[i];
            }
            let nbytes = match &opt {
                Some(st) => st.as_bytes(),
                None => b"",
            };
            for i in 0..nbytes.len() {
                ret[i + 4] = nbytes[i];
            }

            ret
        };

        let b1 = PacketCore::from(BackupCommand::Fire(Some("Dummy".to_string())));
        let b2 = PacketCore::from(BackupCommand::ChangeTarget(None, PathBuf::from("/foo/bar")));
        let b3 = PacketCore::from(BackupCommand::ChangeSource(
            Some("Dummy".to_string()),
            PathBuf::from("/foo/bar/baz"),
        ));
        let b4 = PacketCore::from(BackupCommand::HasHostCapability(None));
        let b5 = PacketCore::from(BackupCommand::ChangeHost(
            Some("Dummy".to_string()),
            [192, 168, 1, 1],
        ));
        let b6 = PacketCore::from(BackupCommand::ChangeHostCredentials(
            None,
            HostCredentials("foo".to_string(), "bar".to_string()),
        ));
        let b7 = PacketCore::from(BackupCommand::PingHost(Some("Dummy".to_string())));
        let b8 = PacketCore::from(BackupCommand::Print(None));
        let b9 = PacketCore::from(BackupCommand::Undef);

        let t1 = genfire(Some("Dummy".to_string()));
        let t2 = frompbuf(b"CHTA", None, PathBuf::from("/foo/bar"));
        let t3 = frompbuf(
            b"CHSR",
            Some("Dummy".to_string()),
            PathBuf::from("/foo/bar/baz"),
        );
        let t4 = gencap(None);
        let t5 = genchhost(&[192, 168, 1, 1], Some("Dummy".to_string()));
        let t6 = gencreds("foo", "bar", None);
        let t7 = genping(Some("Dummy".to_string()));
        let t8 = genprint(None);

        for i in 0..CORE_SIZE {
            assert_eq!(b1[i], t1[i]);
            assert_eq!(b2[i], t2[i]);
            assert_eq!(b3[i], t3[i]);
            assert_eq!(b4[i], t4[i]);
            assert_eq!(b5[i], t5[i]);
            assert_eq!(b6[i], t6[i]);
            assert_eq!(b7[i], t7[i]);
            assert_eq!(b8[i], t8[i]);
            assert_eq!(b9[i], t8[i]);
        }
    }

    #[test]
    fn packet_core_to_logger_command() {
        let h1 = b"WRIT";
        let h2 = b"BWAA";
        let m = b"FOO BAR BAZ";
        let mlen = m.len();
        let mut b1 = [0u8; CORE_SIZE];
        let mut b2 = [0u8; CORE_SIZE];

        for i in 0..4 {
            b1[i] = h1[i];
            b2[i] = h2[i];
        }

        for i in 0..mlen {
            b1[i + 4] = m[i];
            b2[i + 4] = m[i];
        }

        let t1 = LoggerCommand::from(b1);
        let t2 = LoggerCommand::from(b2);

        assert_eq!(t1, LoggerCommand::Write("FOO BAR BAZ".to_string()));
        assert_eq!(t2, LoggerCommand::Undef);
    }

    #[test]
    fn logger_command_to_packet_core() {
        let h1 = b"WRIT";
        let m = b"FOO BAR BAZ";
        let mlen = m.len();
        let mut b1 = [0u8; CORE_SIZE];
        let mut b2 = [0u8; CORE_SIZE];

        for i in 0..4 {
            b1[i] = h1[i];
            b2[i] = h1[i];
        }

        for i in 0..mlen {
            b1[i + 4] = m[i];
        }

        let t1 = PacketCore::from(LoggerCommand::Write("FOO BAR BAZ".to_string()));
        let t2 = PacketCore::from(LoggerCommand::Undef);

        for i in 0..CORE_SIZE {
            assert_eq!(b1[i], t1[i]);
            assert_eq!(b2[i], t2[i]);
        }
    }

    #[test]
    fn alive() {
        let p = Packet::new_alive("foo");
        let res = parse_alive(p);
        match res {
            Ok(s) => assert!(s.eq("foo")),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn notification() {
        let p = Packet::new_ng("foo", "bar", "baz");
        let n = Notification::from(p);
        assert!(n.good);
        assert!(n.message.eq("foo"));
        assert!(n.provider.eq("bar"));
        assert!(n.stage.eq("baz"));
        let p = Packet::new_nw("foo", "bar", "baz");
        let n = Notification::from(p);

        assert!(n.good);
        assert!(n.message.eq("foo"));
        assert!(n.provider.eq("bar"));
        assert!(n.stage.eq("baz"));
        let p = Packet::new_ne("foo", "bar", "baz");
        let n = Notification::from(p);

        assert!(n.good);
        assert!(n.message.eq("foo"));
        assert!(n.provider.eq("bar"));
        assert!(n.stage.eq("baz"));
        let p = Packet::new_alive("foo");
        let n = Notification::from(p);

        assert!(!n.good);
    }
}
