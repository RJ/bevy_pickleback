use bevy::prelude::*;
use pickleback::prelude::*;

use std::{
    io,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

#[derive(Resource)]
pub struct ClientTransport {
    socket: UdpSocket,
    buffer: [u8; 1500],
}

pub struct PicklebackClientPlugin;

impl Plugin for PicklebackClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PicklebackConfig>();
        let config = app.world.get_resource::<PicklebackConfig>().unwrap();
        let client = PicklebackClient::new(0.0, config);
        // client.connect();
        app.insert_resource(client);

        app.insert_resource(
            ClientTransport::new("127.0.0.1:60002").expect("Failed setting up transport"),
        );

        app.add_event::<ClientState>();
        app.add_systems(PreUpdate, Self::update);
        app.add_systems(PostUpdate, Self::send_packets);
    }
}

impl ClientTransport {
    /// Create UDP client transport bound to local_addr
    pub fn new<A: ToSocketAddrs>(local_addr: A) -> io::Result<Self> {
        let socket = UdpSocket::bind(local_addr)?;
        socket
            .set_nonblocking(true)
            .expect("Couldn't set nonblocking");
        Ok(Self {
            socket,
            buffer: [0_u8; 1500],
        })
    }
    /// Binds socket to remote server address
    pub fn connect<A: ToSocketAddrs>(&mut self, server_addr: A) -> io::Result<()> {
        self.socket.connect(server_addr)
    }
    /// Gets remote server address
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.socket.peer_addr()
    }
    /// Gets local address socket is bound to
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    /// Receives a packet, if one is available.
    #[inline]
    pub fn receive_packet(&mut self) -> Option<&[u8]> {
        // because we called socket.connect, we can only send and receive from server addr
        // so no need to verify the source address here.
        let packet = match self.socket.recv_from(&mut self.buffer) {
            Ok((len, _addr)) => &self.buffer[..len],
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return None,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => return None,
            Err(e) => {
                warn!("receive_packet err: {e:?}");
                return None;
            }
        };
        Some(packet)
    }
    /// Sends packet to server
    #[inline]
    pub fn send_packet(&mut self, packet: &[u8]) -> std::result::Result<usize, std::io::Error> {
        self.socket.send(packet)
    }
}

impl PicklebackClientPlugin {
    fn update(
        mut client: ResMut<PicklebackClient>,
        mut transport: ResMut<ClientTransport>,
        time: Res<Time>,
        mut ev_states: EventWriter<ClientState>,
    ) {
        client.update(time.delta_seconds_f64());
        // write state changes to events
        client
            .drain_state_transitions()
            .for_each(|s| ev_states.send(s));
        // receive packets from transport, hand off to pickleback client
        while let Some(packet) = transport.receive_packet() {
            if let Err(e) = client.receive(packet) {
                warn!("client.receive: {e:?}");
            }
        }
    }
    fn send_packets(mut client: ResMut<PicklebackClient>, mut transport: ResMut<ClientTransport>) {
        client.visit_packets_to_send(|packet| {
            if let Err(e) = transport.send_packet(packet) {
                warn!("transport.send_packet: {e:?}");
            }
        });
    }
}
