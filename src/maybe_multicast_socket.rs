use std::{
    net::{Ipv4Addr, SocketAddrV4},
    ops::Deref,
};
use tokio::net::UdpSocket;

pub struct MaybeMulticastReceiver {
    socket: UdpSocket,
    group_and_iface: Option<(Ipv4Addr, Ipv4Addr)>, // (group, iface)
}

impl MaybeMulticastReceiver {
    pub async fn new(
        bind_addr: SocketAddrV4,
        group_and_iface: Option<(Ipv4Addr, Ipv4Addr)>,
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;

        if let Some((group, iface)) = group_and_iface {
            socket.join_multicast_v4(group, iface)?;
        }

        Ok(Self {
            socket,
            group_and_iface,
        })
    }
}

impl Drop for MaybeMulticastReceiver {
    fn drop(&mut self) {
        if let Some((group, iface)) = self.group_and_iface {
            let _ = self.socket.leave_multicast_v4(group, iface);
            println!("Left multicast group {group} on interface {iface}");
        }
    }
}

impl Deref for MaybeMulticastReceiver {
    type Target = UdpSocket;
    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl From<UdpSocket> for MaybeMulticastReceiver {
    fn from(socket: UdpSocket) -> Self {
        Self {
            socket,
            group_and_iface: None,
        }
    }
}
