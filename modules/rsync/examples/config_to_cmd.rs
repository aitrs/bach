use rsync::host::Host;
use rsync::rsynconfig::*;
use std::net::Ipv4Addr;
pub fn main() {
    let confitem = RsynConfigItem {
        ttype: SerdeTargetType {
            directory: None,
            mount: Some(MountPoint {
                device: "/dev/sda1".to_string(),
                path: "/mnt/backup".to_string(),
                oloop: false,
                offset: None,
            }),
        },
        source: Source("/var/log".to_string()),
        source_host: None,
        exclude: Some(Exclude("/etc/exclude".to_string())),
        host: Some(Host::new(
            "nas",
            Ipv4Addr::new(192, 168, 10, 123),
            "admin",
            "password",
        )),
        use_host_name: true,
        day_by_day: true,
        stamp_name: None,
        author: None,
        timeout: Some(360),
    };
    let cmd = confitem.to_cmd();
    println!("{:?}", cmd);
}
