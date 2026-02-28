use std::net::UdpSocket;

use clap::Parser;

use serde::Serialize;

#[derive(Serialize)]
#[allow(non_snake_case)]

struct DiscoveryReply {
    Address: String,
    Id: String,
    Name: String,
    EndpointAddress: Option<String>,
}

/// Guess at the local IP by setting up an outbound connection that sends nothing.
pub fn get_local_ip() -> Result<String, std::io::Error> {
    let sock: UdpSocket = UdpSocket::bind("0.0.0.0:0")?;
    sock.connect("203.0.113.1:0")?;

    Ok(sock.local_addr()?.ip().to_string())
}

/// Jelly Discovery Daemon is a tool for declaring that you are a Jellyfin server
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Address to listen on/bind to
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    /// Port to listen on
    ///
    /// Don't touch this, probably, since this is where clients will announce
    /// to. Provided for debug purposes only.
    #[arg(long, default_value_t = 7359)]
    port: u16,

    /// Display name to show in autodiscovery
    ///
    /// This is essentially cosmetic and doesn't need to match your Jellyfin
    /// server. This will be visible in the Jellyfin client.
    #[arg(long, default_value = "Jelly Disco")]
    name: String,

    /// Server ID to return
    ///
    /// I don't know what this does, to be honest. Doesn't seem to need to
    /// match, so a "valid" (whatever that means) default is provided.
    #[arg(long, default_value = "12345678123456781234567812345678")]
    id: String,

    /// Endpoint Address to return
    ///
    /// I don't know what this does. It's null on my Jellyfin.
    #[arg(long, default_value = None)]
    endpoint: Option<String>,

    /// URL of the server to announce
    ///
    /// If not provided, jellydiscod will guess an appropriate local IP to use
    /// for "this" server, based on what interface routes internet-wards.
    #[arg(long)]
    addr: Option<String>,
}

fn run_service(socket: UdpSocket, reply: String) -> std::io::Result<()> {
    loop {
        let mut buf = [0; 100];
        let (_, client_addr) = socket.recv_from(&mut buf)?;
        match str::from_utf8(&buf) {
            Ok(msg) => {
                println!("Recv'd from {}: {}", client_addr, msg);
                socket.send_to(reply.as_bytes(), client_addr)?;
            }
            Err(e) => println!("Failed to decode: {}", e),
        }
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let jelly_addr = if let Some(addr) = args.addr {
        addr
    } else {
        let ip = match get_local_ip() {
            Ok(ip) => ip,
            Err(e) => {
                eprintln!("Failed to determine local IP automatically. Please specify --addr.");
                return Err(e);
            }
        };
        format!("http://{}:8096", ip)
    };

    let reply = DiscoveryReply {
        Address: jelly_addr,
        Id: args.id,
        Name: args.name,
        EndpointAddress: args.endpoint,
    };
    let reply_precanned = serde_json::to_string(&reply)?;

    let bind_addr = format! {"{}:{}", args.bind, args.port};
    println!("Binding to {}", bind_addr);
    println!("Announcement will be: {}", reply_precanned);
    UdpSocket::bind(bind_addr).and_then(|s| run_service(s, reply_precanned))?;
    Ok(())
}
