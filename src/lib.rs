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

pub struct PicklebackClientPlugin;

impl Plugin for PicklebackClientPlugin {
    fn build(&self, app: &mut App) {
        let config = PicklebackConfig::default();
        let server_addr = "127.0.0.1:5000"
            .parse()
            .expect("failed parsing server addr");
        let mut client = PicklebackClient::new(0.0, &config);
        client.connect(server_addr);
        app.insert_resource(client);
        let socket = UdpSocket::bind("127.0.0.1:60002").expect("couldn't bind");
        socket
            .set_nonblocking(true)
            .expect("Couldn't set nonblocking");
        let transport = ClientTransport {
            socket,
            server_addr,
        };
        app.insert_resource(transport);

        app.add_event::<ClientState>();
        app.add_systems(PreUpdate, Self::update);
        app.add_systems(PreUpdate, Self::send_packets);
    }
}

impl PicklebackClientPlugin {
    fn update(
        mut client: ResMut<PicklebackClient>,
        transport: ResMut<ClientTransport>,
        time: Res<Time>,
        mut ev_states: EventWriter<ClientState>,
    ) {
        let mut buffer = [0_u8; 1500]; // TODO don't allocate each time
        client.update(time.delta_seconds_f64());

        // write state changes to events
        client
            .drain_state_transitions()
            .for_each(|s| ev_states.send(s));

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
    fn send_packets(mut client: ResMut<PicklebackClient>, transport: ResMut<ClientTransport>) {
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

pub struct PicklebackServerPlugin;

impl Plugin for PicklebackServerPlugin {
    fn build(&self, app: &mut App) {
        let config = PicklebackConfig::default();
        let time = 0.0;
        let server = PicklebackServer::new(time, &config);
        let socket = UdpSocket::bind("127.0.0.1:5000").expect("Couldn't bind to server socket");
        socket
            .set_nonblocking(true)
            .expect("Failed setting nonblocking on socket");
        let transport = ServerTransport { socket };
        app.add_event::<ServerEvent>();
        app.insert_resource(transport);
        app.insert_resource(server);

        app.add_systems(PreUpdate, Self::advance);
        app.add_systems(PostUpdate, Self::send);
    }
}

impl PicklebackServerPlugin {
    fn advance(
        transport: ResMut<ServerTransport>,
        mut server: ResMut<PicklebackServer>,
        mut events: EventWriter<ServerEvent>,
        time: Res<Time>,
    ) {
        server.update(time.delta_seconds_f64());
        // write client connect/disconnect events to bevy
        server.drain_server_events().for_each(|e| events.send(e));
        // recv packets
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

    fn send(transport: ResMut<ServerTransport>, mut server: ResMut<PicklebackServer>) {
        server.visit_packets_to_send(|addr, packet| {
            if let Err(e) = transport.socket.send_to(packet, addr) {
                error!("error sending to {addr:?} = {e:?}");
            }
        });
    }
}
