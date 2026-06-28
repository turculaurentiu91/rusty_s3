mod http;

use http::{Verb, parse_request};
use std::collections::HashMap;
use std::io::{Error, Read};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::{
    io,
    net::{TcpListener, TcpStream},
    time::Duration,
};

static MAX_EVENTS: usize = 1024;

//TODO: Refactor into parse_header that does not own the stream or the arenas
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
        (Verb::Get, None, None) => {}            // list buckets
        (Verb::Get, Some(bucket), None) => {}    // list bucket
        (Verb::Put, Some(bucket), None) => {}    // create bucket
        (Verb::Delete, Some(bucket), None) => {} // delete bucket

        (Verb::Get, Some(bucket), Some(path)) => {} // read object
        (Verb::Put, Some(bucket), Some(path)) => {} // upload object
        (Verb::Delete, Some(bucket), Some(path)) => {} // delete object

        _ => panic!("unsupported verb"),
    }

    Ok(())
}

struct PendingReq {
    stream: TcpStream,
    buffer: Vec<u8>,
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9100")?;
    listener.set_nonblocking(true)?;
    let listener_fd = listener.as_raw_fd();
    let epoll_fd = unsafe { libc::epoll_create1(0) };
    if epoll_fd == -1 {
        return Err(std::io::Error::last_os_error());
    }

    let mut conn_ready_ev = libc::epoll_event {
        events: (libc::EPOLLIN) as u32,
        u64: listener_fd as u64,
    };

    unsafe {
        libc::epoll_ctl(
            epoll_fd,
            libc::EPOLL_CTL_ADD,
            listener_fd,
            &mut conn_ready_ev,
        )
    };
    let mut events: Vec<libc::epoll_event> = Vec::with_capacity(MAX_EVENTS);
    let mut requests: HashMap<u64, PendingReq> = HashMap::new();
    loop {
        events.clear();
        let res = unsafe {
            libc::epoll_wait(
                epoll_fd,
                events.as_mut_ptr() as *mut libc::epoll_event,
                MAX_EVENTS as libc::c_int,
                -1,
            )
        };

        if res == -1 {
            break;
        }

        unsafe { events.set_len(res as usize) };

        'eventsLoop: for event in &events {
            let key = event.u64;
            if key == listener_fd as u64 {
                let (stream, _) = listener.accept()?;
                let stream_fd = stream.as_raw_fd();

                let mut stream_ready_ev = libc::epoll_event {
                    events: (libc::EPOLLIN) as u32,
                    u64: stream_fd as u64,
                };

                let res = unsafe {
                    libc::epoll_ctl(
                        epoll_fd,
                        libc::EPOLL_CTL_ADD,
                        stream_fd,
                        &mut stream_ready_ev,
                    )
                };

                if res == -1 {
                    continue;
                }

                let req = PendingReq {
                    stream,
                    buffer: Vec::new(),
                };

                req.stream.set_nonblocking(true)?;

                requests.insert(stream_fd as u64, req);

                // std::thread::spawn(move || {
                //     if let Err(e) = handle_connection(stream) {
                //         eprintln!("Connection error: {e}");
                //     }
                // });
            } else {
                let req = requests.get_mut(&key).unwrap();
                let mut buf = [0; 4096];
                loop {
                    match req.stream.read(&mut buf) {
                        Ok(0) => {
                            requests.remove(&key);
                            continue 'eventsLoop;
                        }
                        Ok(n) => req.buffer.extend_from_slice(&buf[0..n]),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(e) => {
                            eprintln!("Socket error: {}", e);
                            requests.remove(&key);
                            continue 'eventsLoop;
                        }
                    }
                }

                if let Some(pos) = req.buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                    // TODO: parse the headers
                }
            }
        }
    }

    Ok(())
}
