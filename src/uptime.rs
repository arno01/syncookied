/// Functions related to tcp secret reading and updating
use std::io;
use std::net::Ipv4Addr;

pub trait UptimeReader: Send {
    /// returns contents of /proc/tcp_secrets file
    fn read(&self) -> io::Result<Vec<u8>>;
}

pub struct LocalReader;

impl UptimeReader for LocalReader {
    fn read(&self) -> io::Result<Vec<u8>> {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::open("/proc/beget_uptime")
                        .or(File::open("/proc/tcp_secrets"))
                        .unwrap();
        let mut buf = vec![];
        try!(file.read_to_end(&mut buf));
        Ok(buf)
    }
}

/// Receives secrets over udp
pub struct UdpReader {
    addr: String,
}

impl UdpReader {
    pub fn new(addr: String) -> Self {
        UdpReader {
            addr: addr 
        }
    }
}

impl UptimeReader for UdpReader {
    fn read(&self) -> io::Result<Vec<u8>> {
        use std::net::UdpSocket;
        use std::time::Duration;

        let mut buf = vec![0;1024];
        let socket = try!(UdpSocket::bind("0.0.0.0:0"));
        let timeout = Duration::new(1, 0);
        try!(socket.set_read_timeout(Some(timeout)));
        try!(socket.set_write_timeout(Some(timeout)));
        loop {
            socket.send_to(b"YO", self.addr.as_str()).unwrap();
            if let Ok(..) = socket.recv_from(&mut buf[0..]) {
                return Ok(buf);
            }
        }
    }
}

// TODO: parser should probably be split into
// its own function
/// parses tcp_secrets and updates global table
pub fn update(ip: Ipv4Addr, buf: Vec<u8>) {
    use std::io::prelude::*;
    use std::io::BufReader;

    let mut jiffies = 0;
    let mut tcp_cookie_time = 0;
    let mut hz = 300;
    let mut syncookie_secret: [[u32;17];2] = [[0;17];2];

    let reader = BufReader::new(&buf[..]);
    for (idx, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        match idx {
            0 => {
                for (idx, word) in line.split(' ').enumerate() {
                    match idx {
                        0 => { jiffies = word.parse::<u64>().unwrap() },
                        1 => { tcp_cookie_time = word.parse::<u32>().unwrap() },
                        2 => { hz = word.parse::<u32>().unwrap() },
                        _ => {},
                    }
                }
            },
            1 => {
                for (idx, word) in line.split('.').enumerate() {
                    if word == "" {
                        continue;
                    }
                    syncookie_secret[0][idx] = u32::from_str_radix(word, 16).unwrap();
                }
            },
            2 => {
                for (idx, word) in line.split('.').enumerate() {
                    if word == "" {
                        continue;
                    }
                    syncookie_secret[1][idx] = u32::from_str_radix(word, 16).unwrap();
                }
            },
            _ => {},
        }
    }
    //println!("jiffies: {}, tcp_cookie_time: {}, syncookie_secret: {:?}", jiffies, tcp_cookie_time, unsafe { syncookie_secret });
    ::RoutingTable::with_host_config_mut(ip, |hc| {
        use std::ptr;
        hc.tcp_timestamp = jiffies & 0xffffffff;
        hc.tcp_cookie_time = tcp_cookie_time as u64;
        hc.hz = hz;
        unsafe {
            ptr::copy_nonoverlapping(syncookie_secret[0].as_ptr(), hc.syncookie_secret[0 as usize].as_mut_ptr(), 17);
            ptr::copy_nonoverlapping(syncookie_secret[1].as_ptr(), hc.syncookie_secret[1 as usize].as_mut_ptr(), 17);
        }
    });
}

/// main function in "server" mode
pub fn run_server(addr: &str) {
    use std::net::UdpSocket;

    println!("Trying to enable syncookies");
    match ::util::set_syncookies(2) {
        Ok(_) => println!("Syncookies enabled"),
        Err(e) => println!("{}", e),
    }
    println!("Listening on {}", addr);
    let socket = UdpSocket::bind(addr).expect("Cannot bind socket");

    loop {
        let mut buf = [0; 64];
        if let Ok((_,addr)) = socket.recv_from(&mut buf[0..]) {
            if let Ok(buf) = LocalReader.read() {
                socket.send_to(&buf[..], addr).unwrap();
            }
        }
    }
}
