use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::time::Duration;

use scotoma_protocol::{InviteSecret, PeerRole, RoomCode};
use scotoma_sim::{ClientConfig, authenticate, decode_invite_secret, encode_hex};
use zeroize::Zeroizing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let command = arguments
        .next()
        .ok_or("usage: scotoma-sim generate | connect <relay-websocket-url> <room>")?;
    if command == "generate" {
        if arguments.next().is_some() {
            return Err("usage: scotoma-sim generate".into());
        }
        let secret = InviteSecret::generate()?;
        println!("{}", encode_hex(secret.expose_bytes()));
        return Ok(());
    }
    if command != "connect" {
        return Err("usage: scotoma-sim generate | connect <relay-websocket-url> <room>".into());
    }
    let relay_url = arguments
        .next()
        .ok_or("usage: scotoma-sim connect <relay-websocket-url> <room>")?;
    let room_value: u64 = arguments
        .next()
        .ok_or("usage: scotoma-sim connect <relay-websocket-url> <room>")?
        .parse()?;
    if arguments.next().is_some() {
        return Err("usage: scotoma-sim connect <relay-websocket-url> <room>".into());
    }
    let room = RoomCode::new(room_value)?;

    print!("Generated invite secret (64 hex digits; visible input in this CLI prototype): ");
    io::stdout().flush()?;
    let mut secret_input = Zeroizing::new(String::new());
    io::stdin().read_line(&mut secret_input)?;
    let secret = decode_invite_secret(&secret_input)?;

    let mut client = authenticate(ClientConfig {
        relay_url,
        room,
        secret,
        operation_timeout: Duration::from_secs(30),
    })
    .await?;

    println!(
        "peer authenticated (role={:?}, channel={})",
        client.role(),
        encode_hex(
            client
                .handshake_hash()
                .ok_or("authenticated client has no channel binding")?
                .get(..6)
                .ok_or("invalid channel binding")?
        )
    );

    match client.role() {
        PeerRole::Initiator => {
            print!("Message to encrypt and send (visible input in this CLI prototype): ");
            io::stdout().flush()?;
            let mut message = Zeroizing::new(String::new());
            io::stdin().read_line(&mut message)?;
            while message.ends_with(['\r', '\n']) {
                message.pop();
            }
            let counter = client.send_text_and_wait_ack(&message).await?;
            println!("authenticated ACK received for message {counter}");
            client.burn().await?;
            println!("authenticated BURN sent; local session destroyed");
        }
        PeerRole::Responder => {
            println!("waiting for one encrypted message...");
            let message = client.receive_text_and_ack().await?;
            println!("decrypted message: {message}");
            println!("authenticated ACK sent");
            client.wait_for_burn().await?;
            println!("authenticated BURN received; local session destroyed");
        }
    }
    Ok(())
}
