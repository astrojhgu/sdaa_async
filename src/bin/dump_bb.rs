use std::net::{Ipv4Addr, SocketAddrV4};

use clap::Parser;
use futures_util::{StreamExt, pin_mut};
use sdaa_async::{
    maybe_multicast_socket::MaybeMulticastReceiver, pipeline::receive_pkt, utils::as_u8_slice,
};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    net::UdpSocket,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short = 'a', long = "addr", value_name = "ip:port")]
    local_addr: String,

    #[clap(short = 'm', long = "maddr", value_name = "ip")]
    multicast_addr: Option<String>,

    #[clap(short = 'o', long = "out", value_name = "out name")]
    outname: Option<String>,

    #[clap(short = 'p', value_name = "npkts to dump")]
    npkts_to_recv: Option<usize>,

    #[clap(short = 's', value_name = "npkts per file")]
    npkts_per_file: Option<usize>,

    #[clap(short = 'b', value_name = "buffer size in MB")]
    buffer_size_mega_byte: Option<usize>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let addr = args.local_addr.parse::<SocketAddrV4>().unwrap();
    let buffer_size_mega_byte = args.buffer_size_mega_byte.unwrap_or(8);

    let socket = if let Some(mcast_addr_str) = args.multicast_addr {
        let local_iface = addr.ip().clone(); // 改成你网卡的实际 IPv4 地址
        let mcast_addr = mcast_addr_str.parse::<Ipv4Addr>().unwrap();
        assert!(mcast_addr.is_multicast());

        MaybeMulticastReceiver::new(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), addr.port()),
            Some((mcast_addr, local_iface)),
        )
        .await
        .unwrap()
    } else {
        UdpSocket::bind(&addr).await.unwrap().into()
    };

    let mut npkts_received = 0;
    let mut current_file_no = 0;
    let mut current_file_npkts = 0;

    let mut dump_file = if let Some(ref fname) = args.outname {
        Some(BufWriter::with_capacity(
            buffer_size_mega_byte * 1024 * 1024,
            if args.npkts_per_file.is_some() {
                File::create(format!("{fname}{current_file_no}.bin"))
                    .await
                    .expect("failed to create output file")
            } else {
                File::create(fname)
                    .await
                    .expect("failed to create output file")
            },
        ))
    } else {
        None
    };

    let s = receive_pkt(socket);
    pin_mut!(s);
    while let Some(payload) = s.next().await {
        if let Some(f) = dump_file.as_mut() {
            f.write_all(as_u8_slice(&payload.data))
                .await
                .expect("failed to write to dump file");
        }
        npkts_received += 1;
        current_file_npkts += 1;

        if let Some(n) = args.npkts_to_recv
            && npkts_received >= n
        {
            break;
        }

        if let Some(npkts_per_file) = args.npkts_per_file
            && let Some(ref fname) = args.outname
            && current_file_npkts >= npkts_per_file
            && npkts_per_file > 0
        {
            current_file_no += 1;
            current_file_npkts = 0;
            dump_file = Some(BufWriter::with_capacity(
                buffer_size_mega_byte * 1024 * 1024,
                File::create(format!("{fname}{current_file_no}.bin"))
                    .await
                    .expect("failed to create output file"),
            ));
            println!("new file segment created")
        }
    }
}
