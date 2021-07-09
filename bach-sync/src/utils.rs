use std::net::Ipv4Addr;

pub fn str2u8(src: &str) -> [u8; 32] {
    let mut ret: [u8; 32] = [0; 32];
    let b = src.as_bytes();
    let len = src.len();

    if len >= 32 {
        for i in 0..30 {
            ret[i] = b[i];
        }
    } else {
        for i in 0..len {
            ret[i] = b[i];
        }
    }
    ret
}

pub fn str2ip(src: &str) -> Result<Ipv4Addr, std::num::ParseIntError> {
    let mut ipbuf = [0u8; 4];
    let ipit = src.split(".");
    let mut c = 0;
    for i in ipit {
        ipbuf[c] = i.parse()?;
        c = c + 1;
    }

    Ok(Ipv4Addr::from(ipbuf))
}

pub fn ip2str(src: &Ipv4Addr) -> String {
    let buf: [u8; 4] = src.octets();
    let mut ret = String::new();
    for i in 0..3 {
        ret.push_str(&format!("{}", buf[i]));
        if i < 3 {
            ret.push_str(".");
        }
    }

    ret
}
