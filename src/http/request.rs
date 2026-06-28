use super::Headers;
use std::io::Chain;
use std::{collections::HashMap, io, io::Read};

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

pub struct Request<'a> {
    pub verb: Verb,
    pub path: &'a str,
    pub headers: Headers<'a>,
    pub query_params: HashMap<&'a str, &'a str>,
    pub body_start_pos: usize,
    pub body_size: usize,
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

pub fn parse_request<'a>(headers_buf: &'a mut Vec<u8>) -> Option<std::io::Result<Request<'a>>> {
    let body_start_pos = match headers_buf.windows(4).position(|w| w == b"\r\n\r\n") {
        None => return None,
        Some(pos) => pos,
    };

    make_headers_lowercase(headers_buf);

    //TODO: `process_query_params` potentially shrinks the buffer, must update the `body_start_pos`
    let query_params = process_query_params(headers_buf);

    let headers_buf = match std::str::from_utf8(headers_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    {
        Ok(val) => val,
        Err(e) => return Some(Err(e)),
    };

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

    let body_size = match headers
        .get("Content-Length")
        .map(|s| s.parse::<usize>())
        .transpose()
    {
        Ok(Some(size)) => size,
        _ => {
            return Some(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid body size",
            )));
        }
    };

    let query_params = query_params
        .iter()
        .map(|(key, val)| {
            (
                headers_buf[key.0..key.1].as_ref(),
                headers_buf[val.0..val.1].as_ref(),
            )
        })
        .collect::<HashMap<&'a str, &'a str>>();

    let request = Request {
        verb,
        path,
        headers,
        query_params,
        body_start_pos,
        body_size,
    };
    Some(Ok(request))
}
