mod http;

use http::{Verb, parse_request};
use std::io::Error;
use std::{
    net::{TcpListener, TcpStream},
    time::Duration,
};

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut headers_buf = Vec::new();
    let mut body_start = Vec::new();

    let request = parse_request(&mut stream, &mut headers_buf, &mut body_start)?;

    let path_segments = request.path.trim_start_matches('/');
    let path_segments = path_segments
        .split_once('/')
        .map(|(bucket, path)| (Some(bucket), Some(path)))
        .unwrap_or_else(|| {
            if path_segments.is_empty() {
                return (None, None);
            }

            (Some(path_segments), None)
        });

    match (request.verb, path_segments.0, path_segments.1) {
        (Verb::Get, None, None) => {} // list buckets
        (Verb::Get, Some(bucket), None) => {} // list bucket
        (Verb::Put, Some(bucket), None) => {} // create bucket
        (Verb::Delete, Some(bucket), None) => {} // delete bucket

        (Verb::Get, Some(bucket), Some(path)) => {} // read object
        (Verb::Put, Some(bucket), Some(path)) => {} // upload object
        (Verb::Delete, Some(bucket), Some(path)) => {} // delete object

        _ => panic!("unsupported verb")
    }

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
