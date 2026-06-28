use crate::emulator::EmulatorState;
use crate::escpos::parser::EscPosParser;
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

pub async fn start_server(emulator_state: Arc<Mutex<EmulatorState>>) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:9100").await?;
    info!("ESC/POS Emulator server listening on 0.0.0.0:9100");

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("New connection from: {}", addr);
                {
                    let mut state = emulator_state.lock().await;
                    state.active_connections += 1;
                }
                let state = emulator_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, state.clone(), addr).await {
                        error!("Error handling connection from {}: {}", addr, e);
                    }
                    let mut s = state.lock().await;
                    s.active_connections = s.active_connections.saturating_sub(1);
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    emulator_state: Arc<Mutex<EmulatorState>>,
    addr: SocketAddr,
) -> Result<()> {
    let mut parser = EscPosParser::new();
    let device_ip = addr.ip().to_string();

    loop {
        let mut chunk = vec![0u8; 4096];
        match socket.read(&mut chunk).await {
            Ok(0) => {
                info!("Connection closed by client: {}", addr);
                break;
            }
            Ok(n) => {
                // Pass data directly to parser — no redundant outer buffer
                if let Ok(commands) = parser.parse_stream(&chunk[..n]) {
                    for command in commands {
                        info!("Received command from {}: {:?}", device_ip, command);
                        let mut state = emulator_state.lock().await;
                        state.process_command(&command, &device_ip);
                    }
                }
            }
            Err(e) => {
                warn!("Error reading from socket {}: {}", addr, e);
                break;
            }
        }
    }

    // Send acknowledgment
    if let Err(e) = socket.write_all(b"OK\n").await {
        warn!("Failed to send response to {}: {}", addr, e);
    }

    Ok(())
}
