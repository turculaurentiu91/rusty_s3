use std::io::Read;

use super::Headers;

#[derive(PartialEq, Debug)]
pub enum Verb {
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

pub struct Request<'a, R: Read> {
    pub verb: Verb,
    pub path: &'a str,
    pub headers: Headers<'a>,
    pub body_size: usize,
    pub stream: R,
}

pub fn parse_request<'a, R: Read>(
    reader: &'a mut R,
    headers_buf: &'a mut Vec<u8>,
    body_start: &'a mut Vec<u8>,
) -> std::io::Result<Request<'a, &'a mut R>> {
    let mut buffer = [0; 4096];

    while let Ok(n) = reader.read(&mut buffer) {
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

    let headers_buf = std::str::from_utf8(headers_buf)
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

    let headers = Headers::new(
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
        stream: reader,
    };
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut req = b"GET somefile.php HTTP1.1\r\nAuthorisation: bearer test\r\nAccept: application/xml\r\nContent-Length: 0\r\n\r\n".as_ref();
        let mut headers_buf = Vec::new();
        let mut body_rest = Vec::new();

        let req = parse_request(&mut req, &mut headers_buf, &mut body_rest).unwrap();

        assert_eq!(req.verb, Verb::Get);
        assert_eq!(req.path, "somefile.php");
        assert_eq!(req.body_size, 0);

        let auth_header = req.headers.get("authorisation").unwrap();
        assert_eq!(auth_header, "bearer test");

        let accept_header = req.headers.get("Accept").unwrap();
        assert_eq!(accept_header, "application/xml");
    }
}
