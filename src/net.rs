use {bevy::prelude::*, bevy_replicon::prelude::*, bytes::Bytes};

fn framed(channel: usize, message: &[u8]) -> Vec<u8> {
  std::iter::once(channel as u8).chain(message.iter().copied()).collect()
}

fn unframed(frame: Bytes) -> Option<(usize, Bytes)> {
  (!frame.is_empty()).then(|| (frame[0] as usize, frame.slice(1..)))
}

#[cfg(not(target_arch = "wasm32"))]
pub mod server {
  use {super::*,
       bevy_replicon::shared::backend::connected_client::NetworkId,
       std::{io::ErrorKind,
             net::{TcpListener, TcpStream},
             sync::{Mutex,
                    mpsc::{Receiver, Sender, channel}},
             time::Duration},
       tungstenite::{Error, Message, WebSocket}};

  type Socket = WebSocket<TcpStream>;

  #[derive(Resource)]
  pub struct Listener {
    socket: TcpListener,
    greeted: Mutex<Receiver<Socket>>,
    greeter: Sender<Socket>
  }

  impl Listener {
    pub fn open(port: u16) -> std::io::Result<Listener> {
      let socket = TcpListener::bind(("0.0.0.0", port))?;
      socket.set_nonblocking(true)?;
      let (greeter, greeted) = channel();
      info!("listening for websocket players on port {port}");
      Ok(Listener { socket, greeted: Mutex::new(greeted), greeter })
    }
  }

  #[derive(Component)]
  struct Link(Socket);

  fn greet(stream: TcpStream, greeter: Sender<Socket>) {
    std::thread::spawn(move || {
      let accepted = stream
        .set_nonblocking(false)
        .and_then(|()| stream.set_read_timeout(Some(Duration::from_secs(5))))
        .and_then(|()| stream.set_nodelay(true))
        .map_err(|blame| blame.to_string())
        .and_then(|()| tungstenite::accept(stream).map_err(|blame| blame.to_string()))
        .and_then(|socket| {
          socket
            .get_ref()
            .set_nonblocking(true)
            .map(|()| socket)
            .map_err(|blame| blame.to_string())
        });
      match accepted {
        Ok(socket) => drop(greeter.send(socket)),
        Err(blame) => warn!("websocket handshake failed: {blame}")
      }
    });
  }

  fn running(mut state: ResMut<NextState<ServerState>>) {
    state.set(ServerState::Running)
  }

  fn receive(
    mut commands: Commands,
    mut messages: ResMut<ServerMessages>,
    listener: Res<Listener>,
    mut links: Query<(Entity, &mut Link)>,
    mut next_id: Local<u64>
  ) {
    for (stream, _) in std::iter::from_fn(|| listener.socket.accept().ok()) {
      greet(stream, listener.greeter.clone())
    }
    let greeted =
      listener.greeted.lock().map(|greeted| greeted.try_iter().collect::<Vec<_>>());
    for socket in greeted.unwrap_or_default().into_iter() {
      *next_id += 1;
      let client = commands
        .spawn((
          ConnectedClient { max_size: 1 << 16 },
          NetworkId::new(*next_id),
          Link(socket)
        ))
        .id();
      info!("player connected as {client}")
    }
    for (client, mut link) in links.iter_mut() {
      let mut closed = false;
      loop {
        match link.0.read() {
          Ok(Message::Binary(frame)) => {
            if let Some((channel, message)) = unframed(frame) {
              messages.insert_received(client, channel, message)
            }
          }
          Ok(Message::Close(_)) => closed = true,
          Ok(_) => (),
          Err(Error::Io(blame)) if blame.kind() == ErrorKind::WouldBlock => break,
          Err(_) => {
            closed = true;
            break;
          }
        }
      }
      if closed {
        info!("player {client} disconnected");
        commands.entity(client).despawn()
      }
    }
  }

  fn send(
    mut commands: Commands,
    mut messages: ResMut<ServerMessages>,
    mut disconnects: MessageReader<DisconnectRequest>,
    mut links: Query<&mut Link>
  ) {
    for (client, channel, message) in messages.drain_sent() {
      if let Ok(mut link) = links.get_mut(client)
        && let Err(blame) =
          link.0.write(Message::Binary(framed(channel, &message).into()))
        && !matches!(&blame, Error::Io(io) if io.kind() == ErrorKind::WouldBlock)
      {
        commands.entity(client).despawn()
      }
    }
    for mut link in links.iter_mut() {
      drop(link.0.flush())
    }
    for request in disconnects.read() {
      if let Ok(mut link) = links.get_mut(request.client) {
        drop(link.0.close(None));
        drop(link.0.flush())
      }
      commands.entity(request.client).despawn()
    }
  }

  pub struct ServerNet;

  impl Plugin for ServerNet {
    fn build(&self, app: &mut App) {
      app
        .add_systems(
          PreUpdate,
          (
            running.run_if(resource_added::<Listener>),
            receive.run_if(resource_exists::<Listener>)
          )
            .in_set(ServerSystems::ReceivePackets)
        )
        .add_systems(
          PostUpdate,
          send.run_if(resource_exists::<Listener>).in_set(ServerSystems::SendPackets)
        );
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
mod link {
  use {super::*,
       std::{io::ErrorKind,
             net::TcpStream,
             sync::{Mutex,
                    mpsc::{Receiver, channel}}},
       tungstenite::{Error, Message, WebSocket, stream::MaybeTlsStream}};

  type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

  pub enum Link {
    Dialing(Mutex<Receiver<Result<Socket, String>>>),
    Open(Socket),
    Closed
  }

  pub fn dial(address: String) -> Link {
    let (sender, receiver) = channel();
    std::thread::spawn(move || {
      let dialed = tungstenite::connect(address.as_str())
        .map_err(|blame| blame.to_string())
        .and_then(|(socket, _)| match socket.get_ref() {
          MaybeTlsStream::Plain(stream) => stream
            .set_nonblocking(true)
            .and_then(|()| stream.set_nodelay(true))
            .map(|()| socket)
            .map_err(|blame| blame.to_string()),
          _ => Err("only ws:// is supported natively".into())
        });
      drop(sender.send(dialed))
    });
    Link::Dialing(Mutex::new(receiver))
  }

  impl Link {
    pub fn poll(&mut self) -> Vec<(usize, Bytes)> {
      let dialed = match self {
        Link::Dialing(receiver) => {
          receiver.lock().ok().and_then(|receiver| receiver.try_recv().ok())
        }
        _ => None
      };
      match dialed {
        Some(Ok(socket)) => *self = Link::Open(socket),
        Some(Err(blame)) => {
          error!("could not connect: {blame}");
          *self = Link::Closed
        }
        None => ()
      }
      let mut received = Vec::new();
      if let Link::Open(socket) = self {
        let mut closed = false;
        loop {
          match socket.read() {
            Ok(Message::Binary(frame)) => received.extend(unframed(frame)),
            Ok(Message::Close(_)) => closed = true,
            Ok(_) => (),
            Err(Error::Io(blame)) if blame.kind() == ErrorKind::WouldBlock => break,
            Err(blame) => {
              error!("connection lost: {blame}");
              closed = true;
              break;
            }
          }
        }
        if closed {
          *self = Link::Closed
        }
      }
      received
    }

    pub fn send(&mut self, channel: usize, message: &[u8]) {
      if let Link::Open(socket) = self
        && let Err(blame) = socket.write(Message::Binary(framed(channel, message).into()))
        && !matches!(&blame, Error::Io(io) if io.kind() == ErrorKind::WouldBlock)
      {
        error!("connection lost: {blame}");
        *self = Link::Closed
      }
    }

    pub fn flush(&mut self) {
      if let Link::Open(socket) = self {
        drop(socket.flush())
      }
    }

    pub fn open(&self) -> bool { matches!(self, Link::Open(_)) }

    pub fn closed(&self) -> bool { matches!(self, Link::Closed) }
  }
}

#[cfg(target_arch = "wasm32")]
mod link {
  use {super::*,
       std::{cell::RefCell, collections::VecDeque, rc::Rc},
       wasm_bindgen::{JsCast, closure::Closure},
       web_sys::{BinaryType, MessageEvent, WebSocket}};

  enum Happening {
    Opened,
    Frame(Bytes),
    Closed(String)
  }

  pub struct Link {
    socket: Option<WebSocket>,
    inbox: Rc<RefCell<VecDeque<Happening>>>,
    state: State,
    _handlers: Vec<Closure<dyn FnMut(web_sys::Event)>>
  }

  #[derive(PartialEq)]
  enum State {
    Dialing,
    Open,
    Closed
  }

  pub fn dial(address: String) -> Link {
    let inbox = Rc::new(RefCell::new(VecDeque::new()));
    let handler = |inbox: &Rc<RefCell<VecDeque<Happening>>>,
                   happening: fn(web_sys::Event) -> Happening| {
      let inbox = inbox.clone();
      Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        inbox.borrow_mut().push_back(happening(event))
      })
    };
    match WebSocket::new(&address) {
      Ok(socket) => {
        socket.set_binary_type(BinaryType::Arraybuffer);
        let handlers = vec![
          handler(&inbox, |_| Happening::Opened),
          handler(&inbox, |event| {
            let data = event.unchecked_into::<MessageEvent>().data();
            Happening::Frame(Bytes::from(js_sys::Uint8Array::new(&data).to_vec()))
          }),
          handler(&inbox, |_| Happening::Closed("connection closed".into())),
          handler(&inbox, |_| Happening::Closed("connection error".into())),
        ];
        socket.set_onopen(Some(handlers[0].as_ref().unchecked_ref()));
        socket.set_onmessage(Some(handlers[1].as_ref().unchecked_ref()));
        socket.set_onclose(Some(handlers[2].as_ref().unchecked_ref()));
        socket.set_onerror(Some(handlers[3].as_ref().unchecked_ref()));
        Link { socket: Some(socket), inbox, state: State::Dialing, _handlers: handlers }
      }
      Err(blame) => {
        error!("could not open websocket: {blame:?}");
        Link { socket: None, inbox, state: State::Closed, _handlers: Vec::new() }
      }
    }
  }

  impl Link {
    pub fn poll(&mut self) -> Vec<(usize, Bytes)> {
      let happenings: Vec<Happening> = self.inbox.borrow_mut().drain(..).collect();
      happenings
        .into_iter()
        .filter_map(|happening| match happening {
          Happening::Opened => {
            self.state = State::Open;
            None
          }
          Happening::Frame(frame) => unframed(frame),
          Happening::Closed(blame) => {
            error!("{blame}");
            self.state = State::Closed;
            None
          }
        })
        .collect()
    }

    pub fn send(&mut self, channel: usize, message: &[u8]) {
      if let Some(socket) = &self.socket
        && self.state == State::Open
        && socket.send_with_u8_array(&framed(channel, message)).is_err()
      {
        self.state = State::Closed
      }
    }

    pub fn flush(&mut self) {}

    pub fn open(&self) -> bool { self.state == State::Open }

    pub fn closed(&self) -> bool { self.state == State::Closed }
  }
}

pub struct Uplink(link::Link);

fn dial(world: &mut World) {
  if let Some(address) = crate::opts::opts().connect.clone() {
    info!("connecting to {address}");
    world.insert_non_send(Uplink(link::dial(address)));
    world.resource_mut::<NextState<ClientState>>().set(ClientState::Connecting)
  }
}

fn receive(
  uplink: Option<NonSendMut<Uplink>>,
  mut messages: ResMut<ClientMessages>,
  state: Res<State<ClientState>>,
  mut next: ResMut<NextState<ClientState>>
) {
  if let Some(mut uplink) = uplink {
    for (channel, message) in uplink.0.poll().into_iter() {
      messages.insert_received(channel, message)
    }
    match (state.get(), uplink.0.open(), uplink.0.closed()) {
      (ClientState::Connecting, true, _) => next.set(ClientState::Connected),
      (ClientState::Connecting | ClientState::Connected, _, true) => {
        next.set(ClientState::Disconnected)
      }
      _ => ()
    }
  }
}

fn send(uplink: Option<NonSendMut<Uplink>>, mut messages: ResMut<ClientMessages>) {
  if let Some(mut uplink) = uplink {
    for (channel, message) in messages.drain_sent() {
      uplink.0.send(channel, &message)
    }
    uplink.0.flush()
  }
}

pub struct ClientNet;

impl Plugin for ClientNet {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, dial)
      .add_systems(PreUpdate, receive.in_set(ClientSystems::ReceivePackets))
      .add_systems(PostUpdate, send.in_set(ClientSystems::SendPackets));
  }
}
