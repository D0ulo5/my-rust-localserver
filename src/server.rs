use crate::config::Config;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

const LISTENER_START: usize = 1_000_000;

#[derive(PartialEq)]
enum ConnectionState {
    Reading,
    Writing,
}

struct Connection {
    stream: TcpStream,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    state: ConnectionState,
}

pub struct Server {
    config: Config,
    poll: Poll,
    listeners: HashMap<Token, TcpListener>,
    connections: HashMap<Token, Connection>,
    next_token: usize,
}

// Standalone function to build a simple HTTP response.
fn prepare_response(conn: &mut Connection) -> Result<(), String> {
    let body = "Hello from LocalServer!";
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Length: {}\r\n\
         Content-Type: text/plain\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        body.len(),
        body
    );
    conn.write_buf.extend_from_slice(response.as_bytes());
    Ok(())
}

impl Server {
    pub fn new(config: Config) -> Result<Self, String> {
        let poll = Poll::new().map_err(|e| format!("Failed to create Poll: {}", e))?;
        let mut listeners = HashMap::new();
        let mut next_token = LISTENER_START;

        for port in &config.ports {
            let addr = format!("{}:{}", config.host, port);
            let mut listener = TcpListener::bind(addr.parse().unwrap())
                .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

            let token = Token(next_token);
            next_token += 1;

            poll.registry()
                .register(&mut listener, token, Interest::READABLE)
                .map_err(|e| format!("Failed to register listener: {}", e))?;

            listeners.insert(token, listener);
            println!("Listening on {}", addr);
        }

        Ok(Server {
            config,
            poll,
            listeners,
            connections: HashMap::new(),
            next_token,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut events = Events::with_capacity(1024);

        loop {
            match self.poll.poll(&mut events, Some(Duration::from_millis(100))) {
                Ok(_) => {
                    for event in events.iter() {
                        let token = event.token();

                        // Listener events
                        if self.listeners.contains_key(&token) {
                            if let Err(e) = self.accept_connection(token) {
                                eprintln!("Accept error: {}", e);
                            }
                            continue;
                        }

                        // Client connection events
                        if let Some(conn) = self.connections.get_mut(&token) {
                            if event.is_readable() && conn.state == ConnectionState::Reading {
                                if let Err(e) = Self::handle_read(conn) {
                                    eprintln!("Read error on {:?}: {}", token, e);
                                    self.connections.remove(&token);
                                    continue;
                                }
                            }
                            if event.is_writable() && conn.state == ConnectionState::Writing {
                                if let Err(e) = Self::handle_write(conn) {
                                    eprintln!("Write error on {:?}: {}", token, e);
                                    self.connections.remove(&token);
                                    continue;
                                }
                            }
                        } else {
                            eprintln!("Unknown token: {:?}", token);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Poll error: {}", e);
                    // continue running
                }
            }
        }
    }

    fn accept_connection(&mut self, listener_token: Token) -> Result<(), String> {
        let listener = self
            .listeners
            .get_mut(&listener_token)
            .ok_or("Listener not found")?;

        loop {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    let token = Token(self.next_token);
                    self.next_token += 1;

                    self.poll
                        .registry()
                        .register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)
                        .map_err(|e| format!("Failed to register connection: {}", e))?;

                    let conn = Connection {
                        stream,
                        read_buf: Vec::with_capacity(4096),
                        write_buf: Vec::new(),
                        state: ConnectionState::Reading,
                    };
                    self.connections.insert(token, conn);
                    println!("New connection from {} (token {:?})", addr, token);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(format!("Accept error: {}", e)),
            }
        }
        Ok(())
    }

    // Static methods that work on a connection reference (no self borrowing)
    fn handle_read(conn: &mut Connection) -> Result<(), String> {
        let mut buf = [0u8; 4096];
        loop {
            match conn.stream.read(&mut buf) {
                Ok(0) => return Err("Connection closed by client".to_string()),
                Ok(n) => {
                    conn.read_buf.extend_from_slice(&buf[..n]);

                    // Temporary: respond immediately after reading (no actual HTTP parsing yet)
                    prepare_response(conn)?;
                    conn.state = ConnectionState::Writing;
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(format!("Read error: {}", e)),
            }
        }
        Ok(())
    }

    fn handle_write(conn: &mut Connection) -> Result<(), String> {
        if conn.write_buf.is_empty() {
            return Err("No data to write".to_string());
        }

        match conn.stream.write(&conn.write_buf) {
            Ok(n) => {
                conn.write_buf.drain(..n);
                if conn.write_buf.is_empty() {
                    // All data sent - close connection
                    return Err("Response fully sent".to_string());
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Wait for next writable event.
            }
            Err(e) => return Err(format!("Write error: {}", e)),
        }
        Ok(())
    }
}