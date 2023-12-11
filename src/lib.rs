use bevy::prelude::*;
use pickleback::prelude::*;
use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

#[derive(Resource)]
pub struct ClientTransport {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

#[derive(Resource, Deref, DerefMut)]
pub struct Client(PicklebackClient);

pub struct PicklebackClientPlugin;

impl Plugin for PicklebackClientPlugin {
    fn build(&self, app: &mut App) {
        let config = PicklebackConfig::default();
        let server_addr = "127.0.0.1:6000"
            .parse()
            .expect("failed parsing server addr");
        let mut client = PicklebackClient::new(0.0, &config);
        client.connect(server_addr);
        app.insert_resource(Client(client));
        let socket = UdpSocket::bind("127.0.0.1:0").expect("couldn't bind");
        socket
            .set_nonblocking(true)
            .expect("Couldn't set nonblocking");
        let transport = ClientTransport {
            socket,
            server_addr: "127.0.0.1:6000".parse().expect("server addr error"),
        };
        app.insert_resource(transport);
        app.add_systems(PreUpdate, Self::update);
        app.add_systems(PreUpdate, Self::send_packets);
    }
}

impl PicklebackClientPlugin {
    fn update(mut client: ResMut<Client>, transport: ResMut<ClientTransport>, time: Res<Time>) {
        let mut buffer = [0_u8; 1500];
        client.update(time.delta_seconds_f64());

        loop {
            let (packet, source) = match transport.socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    if addr != transport.server_addr {
                        warn!("Packet not from server_addr: {:?}", addr);
                        continue;
                    }
                    (&buffer[..len], addr)
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => {
                    error!("{e:?}");
                    return;
                }
            };
            if let Err(e) = client.receive(packet, source) {
                warn!("{e:?}");
            }
        }
    }
    fn send_packets(mut client: ResMut<Client>, transport: ResMut<ClientTransport>) {
        client.visit_packets_to_send(|addr, packet| {
            if let Err(e) = transport.socket.send_to(packet, addr) {
                error!("{e:?}");
            }
        });
    }
}

// SERVER

#[derive(Resource)]
pub struct ServerTransport {
    socket: UdpSocket,
}

#[derive(Resource, Deref, DerefMut)]
pub struct Server(PicklebackServer);

impl Server {
    pub fn connected_clients(&mut self) -> impl Iterator<Item = &mut ConnectedClient> {
        self.connected_clients_mut()
    }
    pub fn kick_client(&mut self, client_id: u64) {
        self.disconnect_client(client_id);
    }
    pub fn get_messages(
        &mut self,
        channel: u8,
    ) -> impl Iterator<Item = (u64, ReceivedMessagesContainer)> {
        self.connected_clients_mut()
            .map(move |cc| (cc.id(), cc.pickleback.drain_received_messages(channel)))
    }
    pub fn get_acks(
        &mut self,
        channel: u8,
    ) -> impl Iterator<Item = (u64, ReceivedMessagesContainer)> {
        self.connected_clients_mut()
            .map(move |cc| (cc.id(), cc.pickleback.drain_received_messages(channel)))
    }
    pub fn send_message(
        &mut self,
        client_id: u64,
        channel: u8,
        message_payload: &[u8],
    ) -> Result<MessageId, PicklebackError> {
        let Some(cc) = self.get_connected_client_by_salt_mut(client_id) else {
            return Err(PicklebackError::NoSuchClient);
        };
        cc.send_message(channel, message_payload)
    }
    /// Broadcast a message to all connected clients, discarding errors and MessageIds
    pub fn broadcast_message(&mut self, channel: u8, message_payload: &[u8]) {
        for cc in self.connected_clients_mut() {
            let _ = cc.pickleback.send_message(channel, message_payload);
        }
    }
}

pub struct PicklebackServerPlugin;

impl Plugin for PicklebackServerPlugin {
    fn build(&self, app: &mut App) {
        let config = PicklebackConfig::default();
        let time = 0.0;
        let server = PicklebackServer::new(time, &config);
        let socket = UdpSocket::bind("127.0.0.1:6000").expect("Couldn't bind to server socket");
        socket
            .set_nonblocking(true)
            .expect("Failed setting nonblocking on socket");
        let transport = ServerTransport { socket };
        let server = Server(server);
        app.insert_resource(transport);
        app.insert_resource(server);

        app.add_systems(PreUpdate, Self::advance);
        app.add_systems(PostUpdate, Self::send);
    }
}

impl PicklebackServerPlugin {
    fn advance(transport: ResMut<ServerTransport>, mut server: ResMut<Server>, time: Res<Time>) {
        server.update(time.delta_seconds_f64());
        let mut buffer = [0_u8; 1500];
        loop {
            match transport.socket.recv_from(&mut buffer) {
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(ref e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => {
                    error!("err recv_from: {e:?}");
                    return;
                }
                Ok((len, client_addr)) => match server.receive(&buffer[..len], client_addr) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("err server.receive: {e:?}");
                    }
                },
            }
        }
    }

    fn send(transport: ResMut<ServerTransport>, mut server: ResMut<Server>) {
        server.visit_packets_to_send(|addr, packet| {
            if let Err(e) = transport.socket.send_to(packet, addr) {
                error!("error sending to {addr:?} = {e:?}");
            }
        });
    }
}
