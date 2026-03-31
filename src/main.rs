mod http;

use std::{
    net::{TcpListener, TcpStream},
    time::Duration,
};

use http::parse_request;

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut headers_buf = Vec::new();
    let mut body_start = Vec::new();

    let request = parse_request(&mut stream, &mut headers_buf, &mut body_start);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9100")?;

    for stream in listener.incoming() {
        let stream = stream?;
        stream.set_read_timeout(Some(Duration::new(30, 0)))?;

        std::thread::spawn(move || {
            if let Err(e) = handle_connection(stream) {
                eprintln!("Connection error: {e}");
            }
        });
    }

    Ok(())
}
