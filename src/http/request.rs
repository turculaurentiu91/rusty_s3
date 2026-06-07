use std::{collections::HashMap, io::Read};
use std::io::Chain;
use super::Headers;

#[derive(PartialEq, Debug, Clone, Copy)]
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
    pub query_params: HashMap<&'a str, &'a str>,
    pub body_size: usize,
    pub stream: Chain<&'a [u8], R>,
}

fn make_headers_lowercase(req: &mut [u8]) {
    let mut line_start = 0;
    let mut index = 0;
    let mut processed_line = false;
    while index < req.len() {
        if index == 0 {
            index += 1;
            continue;
        }

        let byte = req[index];
        if byte == b'\n' && req[index - 1] == b'\r' {
            line_start = index + 1;
            processed_line = false;
        }

        if byte == b':' && line_start != 0 && !processed_line {
            req[line_start..index].make_ascii_lowercase();
            processed_line = true;
        }

        index += 1;
    }
}

fn process_query_params(req: &mut Vec<u8>) -> Vec<((usize, usize), (usize, usize))> {
    let mut index_read = 0;
    let mut index_write = 0;
    let mut questionmark_found = false;
    let mut last_emp_pos = 0;
    let mut last_eq_pos = 0;
    let mut kv_store: Vec<((usize, usize), (usize, usize))> = Vec::new();

    while index_read < req.len() {
        let mut char = req[index_read];

        if char == b'?' && !questionmark_found {
            questionmark_found = true;
            last_emp_pos = index_read;
        }

        if questionmark_found {
            if char == b' ' {
                let key = (last_emp_pos + 1, last_eq_pos);
                let val = (last_eq_pos + 1, index_write);
                kv_store.push((key, val));
                break;
            } else if char == b'%'
                && index_read + 2 < req.len()
                && let Ok(radix) = str::from_utf8(req[index_read + 1..index_read + 3].as_ref())
                && let Ok(parsed_char) = u8::from_str_radix(radix, 16)
            {
                char = parsed_char;
                index_read += 2;
            } else if char == b'=' {
                last_eq_pos = index_write;
            } else if char == b'&' {
                let key = (last_emp_pos + 1, last_eq_pos);
                let val = (last_eq_pos + 1, index_write);
                kv_store.push((key, val));
                last_emp_pos = index_write;
            }
        }

        req[index_write] = char;

        index_read += 1;
        index_write += 1;
    }

    if index_write < index_read {
        req.drain(index_write..index_read);
    }

    kv_store
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

    make_headers_lowercase(headers_buf);

    let query_params = process_query_params(headers_buf);

    let headers_buf = std::str::from_utf8(headers_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut lines = headers_buf.split("\r\n");

    let first_line = lines.next().unwrap();
    let mut first_line = first_line.split(" ");
    let verb = first_line.next().unwrap();
    let path = first_line.next().unwrap();
    let path = path.split_once('?').unwrap_or((path, "")).0;

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

    let query_params = query_params
        .iter()
        .map(|(key, val)| {
            (
                headers_buf[key.0..key.1].as_ref(),
                headers_buf[val.0..val.1].as_ref(),
            )
        })
        .collect::<HashMap<&'a str, &'a str>>();

    let stream = body_start.as_slice().chain(reader);

    let request = Request {
        verb,
        path,
        headers,
        query_params,
        body_size,
        stream,
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

    #[test]
    fn it_works_with_query_params() {
        let mut req = b"GET somefile.php?key=%20val&key2=%26%3Dval HTTP1.1\r\nAuthorisation: bearer test\r\nAccept: application/xml\r\nContent-Length: 0\r\n\r\n".as_ref();
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

        assert_eq!(req.query_params.get("key"), Some(&" val"));
        assert_eq!(req.query_params.get("key2"), Some(&"&=val"));
    }

    #[test]
    fn test_body_read() {
        use std::io::Read;

        let mut req =
            b"POST /upload?test=test HTTP1.1\r\nContent-Length: 11\r\n\r\nhello world".as_ref();
        let mut headers_buf = Vec::new();
        let mut body_rest = Vec::new();

        let mut req = parse_request(&mut req, &mut headers_buf, &mut body_rest).unwrap();

        assert_eq!(req.verb, Verb::Post);
        assert_eq!(req.body_size, 11);

        // The body bytes were pre-read into body_rest during header parsing.
        // Reading from req.stream (the Chain) should yield them transparently.
        let mut body = Vec::new();
        req.stream.read_to_end(&mut body).unwrap();
        assert_eq!(&body, b"hello world");
    }

    #[test]
    fn test_make_headers_lowercase() {
        let expectation = b"GET somefile.php HTTP1.1\r\nauthorisation: bearer:TEST\r\naccept: application/xml\r\ncontent-length: 0".as_ref();
        let mut req = Vec::from(b"GET somefile.php HTTP1.1\r\nAuthorisation: bearer:TEST\r\nAccept: application/xml\r\nContent-Length: 0");

        make_headers_lowercase(&mut req);

        assert_eq!(expectation, &req[..]);
    }

    #[test]
    fn test_process_query_params() {
        //                         0123456789012345678901234567890123456789012345
        // after decode:          "GET somefile.php?key= val&key1=val&key2=&=val HTTP1.1\r\n..."
        //                                          ^  ^    ^   ^   ^   ^
        //                                          17 21   26  30  34  39
        let mut req = Vec::from(
            b"GET somefile.php?key=%20val&key1=val&key2=%26%3Dval HTTP1.1\r\nAuthorisation: bearer:TEST",
        );

        let kv_indices = process_query_params(&mut req);

        assert_eq!(
            str::from_utf8(&req).unwrap(),
            "GET somefile.php?key= val&key1=val&key2=&=val HTTP1.1\r\nAuthorisation: bearer:TEST"
        );

        // key= val
        assert_eq!(kv_indices[0], ((17, 20), (21, 25)));
        assert_eq!(&req[17..20], b"key");
        assert_eq!(&req[21..25], b" val");

        // key1=val
        assert_eq!(kv_indices[1], ((26, 30), (31, 34)));
        assert_eq!(&req[26..30], b"key1");
        assert_eq!(&req[31..34], b"val");

        // key2=&=val
        assert_eq!(kv_indices[2], ((35, 39), (40, 45)));
        assert_eq!(&req[35..39], b"key2");
        assert_eq!(&req[40..45], b"&=val");
    }
}
