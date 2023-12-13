use std::{
    io::{self, ErrorKind},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

use crate::prelude::*;
use bevy::prelude::*;

#[derive(Resource)]
pub struct ServerTransport {
    socket: UdpSocket,
    buffer: [u8; 1500],
}

pub struct PicklebackServerPlugin;

impl Plugin for PicklebackServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PicklebackConfig>();
        let config = app.world.get_resource::<PicklebackConfig>().unwrap();
        let time = 0.0;
        let server = PicklebackServer::new(time, config);
        let transport = ServerTransport::new("127.0.0.1:5000").expect("Failed binding socket");
        app.add_event::<ServerEvent>();
        app.insert_resource(transport);
        app.insert_resource(server);

        app.add_systems(PreUpdate, Self::advance);
        app.add_systems(PostUpdate, Self::send);
    }
}

impl ServerTransport {
    /// Creates new ServerTransport and binds socket to `listen_addr`
    fn new<A: ToSocketAddrs>(listen_addr: A) -> io::Result<Self> {
        let socket = UdpSocket::bind(listen_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buffer: [0_u8; 1500],
        })
    }
    /// Gets local address socket is bound to
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    /// Receives a packet, if one is available.
    #[inline]
    pub fn receive_packet(&mut self) -> Option<(SocketAddr, &[u8])> {
        match self.socket.recv_from(&mut self.buffer) {
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => None,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => None,
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => None,
            Err(e) => {
                warn!("err recv_from: {e:?}");
                None
            }
            Ok((len, client_addr)) => Some((client_addr, &self.buffer[..len])),
        }
    }
    /// Sends packet
    #[inline]
    pub fn send_packet(
        &mut self,
        packet: &[u8],
        dest: SocketAddr,
    ) -> std::result::Result<usize, std::io::Error> {
        self.socket.send_to(packet, dest)
    }
}

impl PicklebackServerPlugin {
    fn advance(
        mut transport: ResMut<ServerTransport>,
        mut server: ResMut<PicklebackServer>,
        mut events: EventWriter<ServerEvent>,
        time: Res<Time>,
    ) {
        server.update(time.delta_seconds_f64());
        // write client connect/disconnect events to bevy
        server.drain_server_events().for_each(|e| events.send(e));
        // recv packets
        while let Some((client_addr, packet)) = transport.receive_packet() {
            match server.receive(packet, client_addr) {
                Ok(_) => {}
                Err(e) => {
                    warn!("err server.receive: {e:?}");
                }
            }
        }
    }

    fn send(mut transport: ResMut<ServerTransport>, mut server: ResMut<PicklebackServer>) {
        server.visit_packets_to_send(|addr, packet| {
            if let Err(e) = transport.send_packet(packet, addr) {
                warn!("error sending to {addr:?} = {e:?}");
            }
        });
    }
}
