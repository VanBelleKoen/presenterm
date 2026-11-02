//! The private channel used by `--dual` mode.
//!
//! The master binds a random localhost port and the slave connects to it. Messages are newline
//! delimited JSON.

use super::speaker_notes::SpeakerNotesEvent;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, ErrorKind, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_millis(200);
const MAX_PENDING_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command")]
enum DualMessage {
    GoTo { slide: u32, chunk: u32 },
    Exit,
    Heartbeat,
}

impl From<&SpeakerNotesEvent> for DualMessage {
    fn from(event: &SpeakerNotesEvent) -> Self {
        match event {
            SpeakerNotesEvent::GoTo { slide, chunk } => Self::GoTo { slide: *slide, chunk: *chunk },
            SpeakerNotesEvent::Exit => Self::Exit,
        }
    }
}

fn encode(message: &DualMessage) -> Vec<u8> {
    let mut data = serde_json::to_vec(message).expect("serialization failed");
    data.push(b'\n');
    data
}

/// The master side of the channel.
pub(crate) struct DualServer {
    listener: TcpListener,
    slave: Option<TcpStream>,
    last_position: Option<DualMessage>,
    last_heartbeat: Instant,
}

impl DualServer {
    /// Bind to a random port on the loopback interface.
    pub(crate) fn bind() -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, slave: None, last_position: None, last_heartbeat: Instant::now() })
    }

    pub(crate) fn address(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub(crate) fn has_slave(&self) -> bool {
        self.slave.is_some()
    }

    /// Accept pending connections and send heartbeats.
    pub(crate) fn poll(&mut self) {
        self.accept();
        if self.slave.is_some() && self.last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            self.last_heartbeat = Instant::now();
            self.write(&DualMessage::Heartbeat);
        }
    }

    /// Send an event to the slave, if any.
    pub(crate) fn send(&mut self, event: &SpeakerNotesEvent) {
        let message = DualMessage::from(event);
        if matches!(message, DualMessage::GoTo { .. }) {
            self.last_position = Some(message.clone());
        }
        self.accept();
        self.write(&message);
    }

    fn accept(&mut self) {
        loop {
            let stream = match self.listener.accept() {
                Ok((stream, address)) if address.ip().is_loopback() => stream,
                // Only localhost connections are allowed.
                Ok(_) => continue,
                Err(_) => return,
            };
            // Only a single slave is supported.
            if self.slave.is_some() {
                continue;
            }
            if stream.set_nonblocking(false).is_err() || stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err() {
                continue;
            }
            let _ = stream.set_nodelay(true);
            self.slave = Some(stream);
            // Bring the new slave up to date.
            if let Some(position) = self.last_position.clone() {
                self.write(&position);
            }
        }
    }

    fn write(&mut self, message: &DualMessage) {
        let Some(slave) = &mut self.slave else {
            return;
        };
        let result = slave.write_all(&encode(message)).and_then(|_| slave.flush());
        if result.is_err() {
            self.slave = None;
        }
    }
}

/// The slave side of the channel.
pub(crate) struct DualClient {
    stream: TcpStream,
    buffer: Vec<u8>,
    pending: VecDeque<SpeakerNotesEvent>,
    last_seen: Option<Instant>,
    closed: bool,
}

impl DualClient {
    pub(crate) fn connect(address: SocketAddr) -> io::Result<Self> {
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream, buffer: Vec::new(), pending: Default::default(), last_seen: None, closed: false })
    }

    /// Get the next event sent by the master.
    ///
    /// This yields an [SpeakerNotesEvent::Exit] if the master is gone.
    pub(crate) fn try_recv(&mut self) -> io::Result<Option<SpeakerNotesEvent>> {
        // The heartbeat timeout only starts counting once we start polling, as the presentation
        // could take a while to load.
        let last_seen = *self.last_seen.get_or_insert_with(Instant::now);
        self.read_available()?;
        if let Some(event) = self.pending.pop_front() {
            return Ok(Some(event));
        }
        if self.closed || last_seen.elapsed() > HEARTBEAT_TIMEOUT {
            return Ok(Some(SpeakerNotesEvent::Exit));
        }
        Ok(None)
    }

    fn read_available(&mut self) -> io::Result<()> {
        let mut chunk = [0; 1024];
        while !self.closed {
            match self.stream.read(&mut chunk) {
                Ok(0) => self.closed = true,
                Ok(count) => {
                    self.last_seen = Some(Instant::now());
                    self.buffer.extend_from_slice(&chunk[..count]);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => self.closed = true,
            }
        }
        self.parse_lines();
        Ok(())
    }

    fn parse_lines(&mut self) {
        while let Some(position) = self.buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=position).collect();
            match serde_json::from_slice::<DualMessage>(&line) {
                Ok(DualMessage::GoTo { slide, chunk }) => {
                    self.pending.push_back(SpeakerNotesEvent::GoTo { slide, chunk })
                }
                Ok(DualMessage::Exit) => self.pending.push_back(SpeakerNotesEvent::Exit),
                Ok(DualMessage::Heartbeat) | Err(_) => (),
            }
        }
        // Don't let garbage without a newline grow forever.
        if self.buffer.len() > MAX_PENDING_BYTES {
            self.buffer.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    fn recv_eventually(client: &mut DualClient) -> Option<SpeakerNotesEvent> {
        for _ in 0..50 {
            if let Some(event) = client.try_recv().unwrap() {
                return Some(event);
            }
            sleep(Duration::from_millis(20));
        }
        None
    }

    #[test]
    fn encode_decode() {
        let message = DualMessage::from(&SpeakerNotesEvent::GoTo { slide: 3, chunk: 1 });
        let data = encode(&message);
        assert_eq!(*data.last().unwrap(), b'\n');
        assert_eq!(serde_json::from_slice::<DualMessage>(&data).unwrap(), message);
    }

    #[test]
    fn events_flow_to_slave() {
        let mut server = DualServer::bind().unwrap();
        let mut client = DualClient::connect(server.address().unwrap()).unwrap();
        sleep(Duration::from_millis(50));
        server.poll();
        assert!(server.has_slave());

        let event = SpeakerNotesEvent::GoTo { slide: 2, chunk: 4 };
        server.send(&event);
        assert_eq!(recv_eventually(&mut client), Some(event));
        server.send(&SpeakerNotesEvent::Exit);
        assert_eq!(recv_eventually(&mut client), Some(SpeakerNotesEvent::Exit));
    }

    #[test]
    fn late_slave_gets_current_position() {
        let mut server = DualServer::bind().unwrap();
        let event = SpeakerNotesEvent::GoTo { slide: 5, chunk: 0 };
        server.send(&event);
        let mut client = DualClient::connect(server.address().unwrap()).unwrap();
        sleep(Duration::from_millis(50));
        server.poll();
        assert_eq!(recv_eventually(&mut client), Some(event));
    }

    #[test]
    fn master_disappearing_exits_slave() {
        let server = DualServer::bind().unwrap();
        let mut client = DualClient::connect(server.address().unwrap()).unwrap();
        let mut server = server;
        sleep(Duration::from_millis(50));
        server.poll();
        drop(server);
        assert_eq!(recv_eventually(&mut client), Some(SpeakerNotesEvent::Exit));
    }

    #[test]
    fn garbage_is_ignored() {
        let mut server = DualServer::bind().unwrap();
        let mut client = DualClient::connect(server.address().unwrap()).unwrap();
        sleep(Duration::from_millis(50));
        server.poll();
        server.slave.as_mut().unwrap().write_all(b"not json\n").unwrap();
        server.send(&SpeakerNotesEvent::GoTo { slide: 1, chunk: 0 });
        assert_eq!(recv_eventually(&mut client), Some(SpeakerNotesEvent::GoTo { slide: 1, chunk: 0 }));
    }
}
