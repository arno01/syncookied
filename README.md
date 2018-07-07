syncookied
==========

![syncookied logo](https://beget.com/images/syncookied/ddos_beget.png)

[![Build Status](https://travis-ci.org/LTD-Beget/syncookied.svg?branch=master)](https://travis-ci.org/LTD-Beget/syncookied)

`syncookied` emulates linux kernel syncookie functionality by intercepting SYN packets
and sending replies to them using the same cookie generation alghorithm. It can achieve
better performance under SYN flood attacks thanks to kernel bypass (netmap).

Installation
============

1. Install rust (instructions here: https://www.rust-lang.org/en-US/downloads.html)
2. Install `build-essential` and `libpcap-dev` or equivalent package for your distribution
3. Install [netmap](https://github.com/luigirizzo/netmap). Make sure netmap.h / netmap_user.h can be found in /usr/include. Alternative you can point CFLAGS variable to their location: [example](https://github.com/LTD-Beget/syncookied/blob/master/.travis.yml).
4. run `cargo build --release`, resulting binary will be found in target/release/syncookied. 

Note: we use [AVX](https://en.wikipedia.org/wiki/Advanced_Vector_Extensions)-accelerated SHA1 function by default. SSE3 implementation is also available under sse3 feature flag, i.e.:  `cargo build --features=sse3 --no-default-features --release`.

How to run
==========

On server you want to protect
------------------------------
1. Install [tcpsecrets](https://github.com/LTD-Beget/tcpsecrets) linux kernel mode to expose tcp syncookie key and timestamp
2. Start syncookied in `server` mode: `syncookied server <proto://ip:port>`. Running this 
commands automatically starts a TCP or UDP server on specified ip/port and sets `net.ipv4.tcp_syncookies` to 2 on first request.

On server you want to use for packet processing
-----------------------------------------------
1. Install [netmap](https://github.com/luigirizzo/netmap) and make sure it works (pkt-gen)

2. Disable NIC offloading features on the interface you want to use (eth2 here):

   ```
   ethtool -K eth2 gro off gso off tso off lro off rx off tx off 
   ethtool -A eth2 rx off tx off
   ethtool -G eth2 rx 2048 tx 2048
   ```

3. Set up queues and affinities. Here we bind 12 queues to first 12 cpu cores:

   ```
   QUEUES=12
   ethtool -L eth2 combined $QUEUES
   ./set_irq_affinity -x 0-11 eth2
   ```

    set_irq_affinity is available at https://github.com/majek/ixgbe/blob/master/scripts/set_irq_affinity

4. Create hosts.yml file in the working directory, which looks like this
   ```
   - ip: 185.50.25.4
     secrets_addr: udp://192.168.3.231:1488
     mac: 0c:c4:7a:6a:fa:bf
   ```
Here ip is external ip you want to protect, secrets_addr is the address of syncookied server running on protected host, and mac is its MAC address.

5. Run `syncookied -i eth2`. It will print something like this:
   ```
   Configuration: 185.50.25.4 -> c:c4:7a:6a:fa:bf
   interfaces: [Rx: eth2/3c:fd:fe:9f:a8:82, Tx: eth2/3c:fd:fe:9f:a8:82] Cores: 24
   12 Rx rings @ eth2, 12 Tx rings @ eth2 Queue: 1048576
   Starting RX thread for ring 0 at eth2
   Starting TX thread for ring 0 at eth2
   Uptime reader for 185.50.25.4 starting
   ...
   ```
6. Configure your network equipment to direct traffic for protected ip to syncookied.

7. You can reload configuration at any time by changing hosts.yml and sending HUP signal to syncookied. 
It will print something like this:

   ```
   Uptime reader for 185.50.25.4 exiting
   All uptime readers dead
   Old readers are dead, all hail to new readers
   Uptime reader for 185.50.25.4 starting
   ...
   ```

8. Enjoy your ddos protection

Notes
-----
`syncookied` has some options you may want to tune, see `syncookied --help`.
If you have more than 1 interface on your server, you may want to look into -O to use second one for TX. 
This greatly improves performance and latency as forwarding and syn-reply traffic is separated.

Traffic filtering
-----------------
It's possible to filter traffic by adding "filters" section to host configuration like this:
```
- ip: 185.50.25.4
  secrets_addr: 127.0.0.1:1488
  mac: 0c:c4:7a:6b:0a:78
  filters:
   tcp and dst port 53: drop
   tcp and dst port 22: pass
   default: pass
```
Filters are written in pcap syntax. Consult `pcap-filter(7)` for more information. 
Default policy is "pass". It can be changed by using `default` key.
Note that filtering happens on layer 4.

Troubleshooting
---------------
Please check the [FAQ](https://github.com/LTD-Beget/syncookied/wiki) before filing an issue.

Need help?
----------
Join us on Telegram: https://telegram.me/syncookied

Performance
===========
syncookied under 12.65 Mpps syn flood attack utilizing 12 cores of Xeon E5-2680v3:
![syncookied perf](http://i.imgur.com/Y5HhQmh.png)

License
=======
`syncookied` is distributed under the term of GPLv2.
