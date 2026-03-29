use std::{
    collections::HashMap,
    io::prelude::*,
    net::{TcpListener, TcpStream},
};

enum Verb {
    Options,
    Head,
    Get,
    Post,
    Patch,
    Put,
    Delete,
    Connect,
    Trace,
    Unknown,
}

struct Headers<'a>(HashMap<&'a str, &'a str>);

impl<'a> Headers<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| *v)
    }
}

struct Request<'a> {
    verb: Verb,
    path: &'a str,
    headers: Headers<'a>,
    body_size: usize,
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9100")?;

    for stream in listener.incoming() {
        let mut buffer = [0; 4096];
        let mut stream = stream?;
        let mut headers_buf = Vec::new();
        let mut body_start = Vec::new();

        while let Ok(n) = stream.read(&mut buffer) {
            if n == 0 {
                break;
            }

            headers_buf.extend_from_slice(&buffer[..n]);

            let search_start = headers_buf.len().saturating_sub(n + 3);

            if let Some(pos) = headers_buf[search_start..]
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
            {
                let actual_pos = search_start + pos;
                body_start.extend_from_slice(&headers_buf[actual_pos + 4..]);
                headers_buf.truncate(actual_pos);
                break;
            }
        }

        let headers_buf = String::from_utf8(headers_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut lines = headers_buf.split("\r\n");

        let first_line = lines.next().unwrap();
        let mut first_line = first_line.split(" ");
        let verb = first_line.next().unwrap();
        let path = first_line.next().unwrap();

        let verb = match verb {
            "OPTIONS" => Verb::Options,
            "HEAD" => Verb::Head,
            "GET" => Verb::Get,
            "POST" => Verb::Post,
            "PUT" => Verb::Put,
            "PATCH" => Verb::Patch,
            "DELETE" => Verb::Delete,
            "TRACE" => Verb::Trace,
            "CONNECT" => Verb::Connect,
            _ => Verb::Unknown,
        };

        let headers = Headers(
            lines
                .filter_map(|line| line.split_once(":"))
                .map(|(k, v)| (k.trim(), v.trim()))
                .collect(),
        );

        let body_size: usize = headers
            .get("Content-Length")
            .map(|s| s.parse())
            .transpose()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            .unwrap_or(0);

        let request = Request {
            verb,
            path,
            headers,
            body_size,
        };
    }

    Ok(())
}
