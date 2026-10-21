mod request;
mod response;

pub(crate) use request::Head;
pub(crate) use response::Response;

use request::Body;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const HANG_UP_WAIT: Duration = Duration::from_secs(1);
const MAX_FRAMING_BYTES: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
const READ_CHUNK: usize = 16 * 1024;
const FRAMING_TOO_LARGE: &str = "Слишком длинный заголовок запроса";
const BAD_CHUNK: &str = "Некорректное chunked-тело запроса";
const BODY_TOO_LARGE: &str = "Слишком большой запрос";

pub enum Failure {
    Gone,
    Refused(Response),
}

impl From<io::Error> for Failure {
    fn from(_: io::Error) -> Self {
        Self::Gone
    }
}

pub fn refused(status: u16, message: &str) -> Failure {
    Failure::Refused(Response::text(status, message))
}

pub struct Connection<'a> {
    stream: &'a TcpStream,
    received: Vec<u8>,
}

impl<'a> Connection<'a> {
    pub fn new(stream: &'a TcpStream) -> io::Result<Self> {
        stream.set_read_timeout(Some(IDLE_TIMEOUT))?;
        stream.set_write_timeout(Some(IDLE_TIMEOUT))?;
        Ok(Self {
            stream,
            received: Vec::new(),
        })
    }

    pub fn read_head(&mut self) -> Result<Head, Failure> {
        loop {
            if let Some(head) = self.parse_head()? {
                return Ok(head);
            }
            self.receive_framing()?;
        }
    }

    pub fn read_body(&mut self, head: &Head, limit: u64) -> Result<Vec<u8>, Failure> {
        let body = head.body()?;
        if matches!(body, Body::Sized(length) if length > limit) {
            return Err(refused(413, BODY_TOO_LARGE));
        }
        if head.expects_continue() {
            self.write(b"HTTP/1.1 100 Continue\r\n\r\n")?;
        }
        match body {
            Body::Sized(length) => Ok(self.take(length as usize)?),
            Body::Chunked => self.read_chunks(limit),
        }
    }

    pub fn send(&mut self, response: &Response, keep_alive: bool) -> io::Result<()> {
        self.write(&response.to_bytes(keep_alive))
    }

    pub fn hang_up(self) {
        let mut stream = self.stream;
        let deadline = Instant::now() + HANG_UP_WAIT;
        if stream.set_read_timeout(Some(HANG_UP_WAIT)).is_err() {
            return;
        }
        let mut discarded = [0; READ_CHUNK];
        while Instant::now() < deadline
            && matches!(stream.read(&mut discarded), Ok(read) if read > 0)
        {}
    }

    fn parse_head(&mut self) -> Result<Option<Head>, Failure> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);
        let length = match request.parse(&self.received) {
            Ok(httparse::Status::Complete(length)) => length,
            Ok(httparse::Status::Partial) => return Ok(None),
            Err(httparse::Error::TooManyHeaders) => return Err(refused(431, FRAMING_TOO_LARGE)),
            Err(_) => return Err(refused(400, "Некорректный запрос")),
        };
        let head = Head::from_parsed(&request);
        self.received.drain(..length);
        Ok(Some(head))
    }

    fn read_chunks(&mut self, limit: u64) -> Result<Vec<u8>, Failure> {
        let mut body = Vec::new();
        loop {
            let size = self.chunk_size()?;
            if size == 0 {
                self.skip_trailers()?;
                return Ok(body);
            }
            if size > limit - body.len() as u64 {
                return Err(refused(413, BODY_TOO_LARGE));
            }
            let chunk = self.take(size as usize + 2)?;
            let Some(data) = chunk.strip_suffix(b"\r\n") else {
                return Err(refused(400, BAD_CHUNK));
            };
            body.extend_from_slice(data);
        }
    }

    fn chunk_size(&mut self) -> Result<u64, Failure> {
        loop {
            match httparse::parse_chunk_size(&self.received) {
                Ok(httparse::Status::Complete((start, size))) => {
                    self.received.drain(..start);
                    return Ok(size);
                }
                Ok(httparse::Status::Partial) => self.receive_framing()?,
                Err(_) => return Err(refused(400, BAD_CHUNK)),
            }
        }
    }

    fn skip_trailers(&mut self) -> Result<(), Failure> {
        loop {
            match self.received.windows(2).position(|pair| pair == b"\r\n") {
                Some(0) => {
                    self.received.drain(..2);
                    return Ok(());
                }
                Some(end) => {
                    self.received.drain(..end + 2);
                }
                None => self.receive_framing()?,
            }
        }
    }

    fn receive_framing(&mut self) -> Result<(), Failure> {
        if self.received.len() >= MAX_FRAMING_BYTES {
            return Err(refused(431, FRAMING_TOO_LARGE));
        }
        Ok(self.receive()?)
    }

    fn take(&mut self, length: usize) -> io::Result<Vec<u8>> {
        self.received
            .reserve(length.saturating_sub(self.received.len()));
        while self.received.len() < length {
            self.receive()?;
        }
        let rest = self.received.split_off(length);
        Ok(std::mem::replace(&mut self.received, rest))
    }

    fn receive(&mut self) -> io::Result<()> {
        let mut chunk = [0; READ_CHUNK];
        let mut stream = self.stream;
        let read = loop {
            match stream.read(&mut chunk) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                outcome => break outcome?,
            }
        };
        if read == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.received.extend_from_slice(&chunk[..read]);
        Ok(())
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut stream = self.stream;
        stream.write_all(bytes)
    }
}
