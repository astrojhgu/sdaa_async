use futures_util::{StreamExt, pin_mut};
use sdaa_async::{maybe_multicast_socket::MaybeMulticastReceiver, pipeline::receive_pkt};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket: MaybeMulticastReceiver =
        UdpSocket::bind("192.168.10.11:3000").await.unwrap().into();
    let s = receive_pkt(socket);
    pin_mut!(s);
    while let Some(payload) = s.next().await {}
}
