use crate::http::{refused, Connection, Failure, Head, Response};
use super::protocol;
use serde_json::Value;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CONNECTIONS: usize = 16;

pub type SharedToolCaller = Arc<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

pub struct RunningServer {
    address: SocketAddr,
    endpoint: Arc<Endpoint>,
    accepting: JoinHandle<()>,
}

struct Endpoint {
    token: String,
    call: SharedToolCaller,
    stopping: AtomicBool,
    turn: Mutex<()>,
    open: Mutex<usize>,
    freed: Condvar,
}

struct Slot(Arc<Endpoint>);

impl Slot {
    fn wait(endpoint: &Arc<Endpoint>) -> Option<Self> {
        let mut open = lock(&endpoint.open);
        while *open >= MAX_CONNECTIONS && !endpoint.stopping.load(Ordering::SeqCst) {
            open = endpoint.freed.wait(open).unwrap_or_else(PoisonError::into_inner);
        }
        if endpoint.stopping.load(Ordering::SeqCst) {
            return None;
        }
        *open += 1;
        Some(Self(Arc::clone(endpoint)))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        *lock(&self.0.open) -= 1;
        self.0.freed.notify_one();
    }
}

impl RunningServer {
    pub fn start(port: u16, token: String, call: SharedToolCaller) -> Result<Self, String> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .map_err(|error| error.to_string())?;
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        let endpoint = Arc::new(Endpoint {
            token,
            call,
            stopping: AtomicBool::new(false),
            turn: Mutex::new(()),
            open: Mutex::new(0),
            freed: Condvar::new(),
        });
        let accepted = Arc::clone(&endpoint);
        let accepting = thread::Builder::new()
            .name("aquilum-mcp".to_owned())
            .spawn(move || accept(&listener, &accepted))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            address,
            endpoint,
            accepting,
        })
    }

    pub fn port(&self) -> u16 {
        self.address.port()
    }

    pub fn stop(self) {
        self.endpoint.stopping.store(true, Ordering::SeqCst);
        drop(lock(&self.endpoint.open));
        self.endpoint.freed.notify_all();
        let _ = TcpStream::connect(self.address);
        let _ = self.accepting.join();
        drop(lock(&self.endpoint.turn));
    }
}

impl Endpoint {
    fn answer(
        &self,
        body: &str,
        connection: &mut Connection,
        keep_alive: bool,
    ) -> Result<bool, Failure> {
        let _turn = lock(&self.turn);
        if self.stopping.load(Ordering::SeqCst) {
            return Err(refused(503, "MCP-сервер остановлен"));
        }
        let response = match protocol::handle_message(body, self.call.as_ref()) {
            Some(payload) => Response::json(payload),
            None => Response::text(202, ""),
        };
        connection.send(&response, keep_alive)?;
        Ok(keep_alive)
    }
}

fn accept(listener: &TcpListener, endpoint: &Arc<Endpoint>) {
    for stream in listener.incoming() {
        if endpoint.stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let Some(slot) = Slot::wait(endpoint) else {
            return;
        };
        let _ = thread::Builder::new()
            .name("aquilum-mcp-connection".to_owned())
            .spawn(move || serve(&stream, &slot.0));
    }
}

fn serve(stream: &TcpStream, endpoint: &Endpoint) {
    let Ok(mut connection) = Connection::new(stream) else {
        return;
    };
    let closing = loop {
        match exchange(&mut connection, endpoint) {
            Ok(true) => {}
            Ok(false) => break Ok(()),
            Err(Failure::Refused(response)) => break connection.send(&response, false),
            Err(Failure::Gone) => return,
        }
    };
    if closing.is_ok() {
        connection.hang_up();
    }
}

fn exchange(connection: &mut Connection, endpoint: &Endpoint) -> Result<bool, Failure> {
    let head = connection.read_head()?;
    admit(&head, &endpoint.token)?;
    let body = connection.read_body(&head, MAX_BODY_BYTES)?;
    let body = String::from_utf8(body).map_err(|_| refused(400, "Тело запроса не в UTF-8"))?;
    endpoint.answer(&body, connection, head.keeps_alive())
}

fn admit(head: &Head, token: &str) -> Result<(), Failure> {
    if head.method != "POST" {
        let response = Response::text(405, "Поддерживается только POST").allowing("POST");
        return Err(Failure::Refused(response));
    }
    if !head.header("origin").is_none_or(is_loopback_origin) {
        return Err(refused(403, "Источник запроса запрещён"));
    }
    if !authorized(head, token) {
        return Err(refused(401, "Неверный токен"));
    }
    Ok(())
}

fn is_loopback_origin(origin: &str) -> bool {
    let value = origin.trim().to_ascii_lowercase();
    let Some(authority) = value.strip_prefix("http://") else {
        return false;
    };
    if authority.contains('/') || authority.contains('@') {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((host, port)) if port.is_empty() || is_port(port.trim_start_matches(':')) => host,
            _ => return false,
        },
        None => match authority.split_once(':') {
            Some((host, port)) if is_port(port) => host,
            Some(_) => return false,
            None => authority,
        },
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn is_port(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn authorized(head: &Head, token: &str) -> bool {
    head.header("authorization")
        .and_then(|value| value.trim().split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .is_some_and(|(_, credentials)| equal_in_constant_time(credentials.trim(), token))
}

fn equal_in_constant_time(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{self, Read, Write};
    use std::net::Shutdown;
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;
    use std::time::Duration;

    const TOKEN: &str = "test-token-123";

    fn start(call: SharedToolCaller) -> (RunningServer, String) {
        let server = RunningServer::start(0, TOKEN.to_owned(), call).unwrap();
        let url = format!("http://127.0.0.1:{}/mcp", server.port());
        (server, url)
    }

    fn server() -> (RunningServer, String) {
        start(Arc::new(|name: &str, _arguments: &Value| {
            Ok(json!({ "echo": name }))
        }))
    }

    fn slow_tool(pause: Duration) -> SharedToolCaller {
        Arc::new(move |_name: &str, _arguments: &Value| {
            thread::sleep(pause);
            Ok(Value::Null)
        })
    }

    fn initialize() -> String {
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }).to_string()
    }

    fn tool_call(name: &str) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": name, "arguments": {} },
        })
        .to_string()
    }

    fn bearer() -> String {
        format!("Bearer {TOKEN}")
    }

    fn status(outcome: Result<ureq::Response, ureq::Error>) -> Option<u16> {
        match outcome {
            Ok(response) => Some(response.status()),
            Err(ureq::Error::Status(status, _)) => Some(status),
            Err(_) => None,
        }
    }

    fn head(extra: &str) -> String {
        format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TOKEN}\r\n{extra}\r\n")
    }

    fn sized(body: &str, extra: &str) -> String {
        let head = head(&format!("Content-Length: {}\r\n{extra}", body.len()));
        format!("{head}{body}")
    }

    struct Answer {
        status: u16,
        connection: String,
        body: String,
    }

    struct Client {
        stream: TcpStream,
        received: Vec<u8>,
    }

    impl Client {
        fn connect(server: &RunningServer) -> Self {
            let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, server.port())).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            Self {
                stream,
                received: Vec::new(),
            }
        }

        fn send(&mut self, text: &str) {
            self.stream.write_all(text.as_bytes()).unwrap();
        }

        fn answer(&mut self) -> Answer {
            loop {
                if let Some(answer) = self.parse() {
                    return answer;
                }
                let mut chunk = [0; 4096];
                let read = self.stream.read(&mut chunk).unwrap();
                assert!(read > 0, "сервер закрыл соединение, не ответив");
                self.received.extend_from_slice(&chunk[..read]);
            }
        }

        fn parse(&mut self) -> Option<Answer> {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut response = httparse::Response::new(&mut headers);
            let httparse::Status::Complete(head) = response.parse(&self.received).unwrap() else {
                return None;
            };
            let header = |name: &str| {
                response
                    .headers
                    .iter()
                    .find(|header| header.name.eq_ignore_ascii_case(name))
                    .map(|header| String::from_utf8_lossy(header.value).into_owned())
                    .unwrap_or_default()
            };
            let length = header("content-length").parse().unwrap_or(0);
            if self.received.len() < head + length {
                return None;
            }
            let answer = Answer {
                status: response.code.unwrap(),
                connection: header("connection"),
                body: String::from_utf8_lossy(&self.received[head..head + length]).into_owned(),
            };
            self.received.drain(..head + length);
            Some(answer)
        }

        fn hung_up(&mut self) -> bool {
            match self.stream.read(&mut [0; 1]) {
                Ok(read) => read == 0,
                Err(error) => !matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ),
            }
        }
    }

    #[test]
    fn answers_an_authorized_request_with_json() {
        let (server, url) = server();
        let response = ureq::post(&url)
            .set("Authorization", &bearer())
            .send_string(&initialize())
            .unwrap();
        assert_eq!(response.status(), 200);
        assert!(response.header("Content-Type").unwrap().contains("application/json"));
        let body: Value = serde_json::from_str(&response.into_string().unwrap()).unwrap();
        assert_eq!(body["result"]["serverInfo"]["name"], "aquilum");
        server.stop();
    }

    #[test]
    fn rejects_a_request_without_a_valid_token() {
        let (server, url) = server();
        let anonymous = ureq::post(&url).send_string(&initialize()).unwrap_err();
        let wrong = ureq::post(&url)
            .set("Authorization", "Bearer another-token")
            .send_string(&initialize())
            .unwrap_err();
        assert!(matches!(anonymous, ureq::Error::Status(401, _)));
        assert!(matches!(wrong, ureq::Error::Status(401, _)));
        server.stop();
    }

    #[test]
    fn accepts_the_bearer_scheme_in_any_case() {
        let (server, url) = server();
        let response = ureq::post(&url)
            .set("Authorization", &format!("BEARER {TOKEN}"))
            .send_string(&initialize())
            .unwrap();
        assert_eq!(response.status(), 200);
        server.stop();
    }

    #[test]
    fn rejects_a_foreign_origin_even_with_a_valid_token() {
        let (server, url) = server();
        let error = ureq::post(&url)
            .set("Authorization", &bearer())
            .set("Origin", "https://evil.example")
            .send_string(&initialize())
            .unwrap_err();
        assert!(matches!(error, ureq::Error::Status(403, _)));
        server.stop();
    }

    #[test]
    fn accepts_a_notification_without_a_body() {
        let (server, url) = server();
        let response = ureq::post(&url)
            .set("Authorization", &bearer())
            .send_string(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string())
            .unwrap();
        assert_eq!(response.status(), 202);
        server.stop();
    }

    #[test]
    fn refuses_methods_other_than_post() {
        let (server, url) = server();
        let error = ureq::get(&url)
            .set("Authorization", &bearer())
            .call()
            .unwrap_err();
        let ureq::Error::Status(405, response) = error else {
            panic!("ожидался 405, пришло {error}");
        };
        assert_eq!(response.header("Allow"), Some("POST"));
        server.stop();
    }

    #[test]
    fn accepts_only_real_loopback_origins() {
        assert!(is_loopback_origin("http://127.0.0.1:1420"));
        assert!(is_loopback_origin("http://localhost"));
        assert!(is_loopback_origin("http://[::1]:8080"));
        assert!(!is_loopback_origin("http://localhost.evil.example"));
        assert!(!is_loopback_origin("http://127.0.0.1.evil.example"));
        assert!(!is_loopback_origin("http://[::1]@evil.example"));
        assert!(!is_loopback_origin("https://localhost"));
        assert!(!is_loopback_origin("null"));
    }

    #[test]
    fn rejects_an_origin_that_only_looks_like_loopback() {
        let (server, url) = server();
        let error = ureq::post(&url)
            .set("Authorization", &bearer())
            .set("Origin", "http://localhost.evil.example")
            .send_string(&initialize())
            .unwrap_err();
        assert!(matches!(error, ureq::Error::Status(403, _)));
        server.stop();
    }

    #[test]
    fn answers_every_short_lived_connection_while_a_tool_is_slow() {
        let (server, url) = start(slow_tool(Duration::from_millis(20)));
        for attempt in 0..40 {
            let response = ureq::post(&url)
                .set("Authorization", &bearer())
                .set("Connection", "close")
                .send_string(&tool_call("slow"))
                .unwrap_or_else(|error| panic!("запрос {attempt}: {error}"));
            assert_eq!(response.status(), 200);
            assert_eq!(response.header("Connection"), Some("close"));
        }
        server.stop();
    }

    #[test]
    fn holds_connections_over_the_limit_until_a_slot_frees() {
        let (server, _) = server();
        let mut held = (0..MAX_CONNECTIONS)
            .map(|_| {
                let mut client = Client::connect(&server);
                client.send(&sized(&tool_call("held"), ""));
                assert_eq!(client.answer().status, 200);
                client
            })
            .collect::<Vec<_>>();

        let mut waiting = Client::connect(&server);
        waiting.send(&sized(&tool_call("waiting"), ""));
        waiting.stream.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
        let mut chunk = [0; 1];
        let early = waiting.stream.read(&mut chunk).unwrap_err().kind();
        assert!(matches!(early, io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut));

        drop(held.pop());
        waiting.stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let answer = waiting.answer();
        assert_eq!(answer.status, 200);
        assert!(answer.body.contains("waiting"));
        drop(held);
        server.stop();
    }

    #[test]
    fn keeps_the_connection_between_requests() {
        let (server, _) = server();
        let mut client = Client::connect(&server);
        for name in ["first", "second"] {
            client.send(&sized(&tool_call(name), ""));
            let answer = client.answer();
            assert_eq!((answer.status, answer.connection.as_str()), (200, "keep-alive"));
            assert!(answer.body.contains(name));
        }
        server.stop();
    }

    #[test]
    fn tells_a_pooled_client_how_long_an_idle_connection_lives() {
        let (server, url) = server();
        let agent = ureq::agent();
        for _ in 0..20 {
            let response = agent
                .post(&url)
                .set("Authorization", &bearer())
                .send_string(&initialize())
                .unwrap();
            assert_eq!(response.header("Keep-Alive"), Some("timeout=60"));
            response.into_string().unwrap();
        }
        server.stop();
    }

    #[test]
    fn closes_the_connection_when_the_client_asks() {
        let (server, _) = server();
        let mut client = Client::connect(&server);
        client.send(&sized(&initialize(), "Connection: close\r\n"));
        let answer = client.answer();
        assert_eq!((answer.status, answer.connection.as_str()), (200, "close"));
        let _ = client.stream.shutdown(Shutdown::Write);
        assert!(client.hung_up());
        server.stop();
    }

    #[test]
    fn waits_for_the_body_after_100_continue() {
        let (server, _) = server();
        let mut client = Client::connect(&server);
        let body = initialize();
        client.send(&head(&format!("Content-Length: {}\r\nExpect: 100-continue\r\n", body.len())));
        assert_eq!(client.answer().status, 100);
        let (first, rest) = body.split_at(10);
        client.send(first);
        thread::sleep(Duration::from_millis(20));
        client.send(rest);
        assert!(client.answer().body.contains("serverInfo"));
        server.stop();
    }

    #[test]
    fn reads_a_chunked_body_with_trailers() {
        let (server, _) = server();
        let mut client = Client::connect(&server);
        let body = initialize();
        let (first, rest) = body.split_at(10);
        client.send(&format!(
            "{}{:x}\r\n{first}\r\n{:x}\r\n{rest}\r\n0\r\nX-Trailer: yes\r\n\r\n",
            head("Transfer-Encoding: chunked\r\n"),
            first.len(),
            rest.len()
        ));
        assert!(client.answer().body.contains("serverInfo"));
        server.stop();
    }

    #[test]
    fn refuses_a_chunked_body_over_the_limit() {
        let (server, url) = server();
        let endless = io::repeat(b' ').take(MAX_BODY_BYTES + 1);
        let error = ureq::post(&url)
            .set("Authorization", &bearer())
            .send(endless)
            .unwrap_err();
        let ureq::Error::Status(413, response) = error else {
            panic!("ожидался 413, пришло {error}");
        };
        assert_eq!(response.header("Connection"), Some("close"));
        server.stop();
    }

    #[test]
    fn answers_an_oversized_body_instead_of_resetting() {
        let (server, url) = server();
        let error = ureq::post(&url)
            .set("Authorization", &bearer())
            .send_string(&"x".repeat(MAX_BODY_BYTES as usize + 1))
            .unwrap_err();
        assert!(matches!(error, ureq::Error::Status(413, _)), "{error}");
        server.stop();
    }

    #[test]
    fn runs_one_tool_call_at_a_time() {
        let running = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let (tool_running, tool_most) = (Arc::clone(&running), Arc::clone(&most));
        let (server, url) = start(Arc::new(move |_name: &str, _arguments: &Value| {
            let now = tool_running.fetch_add(1, Ordering::SeqCst) + 1;
            tool_most.fetch_max(now, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(15));
            tool_running.fetch_sub(1, Ordering::SeqCst);
            Ok(Value::Null)
        }));
        let clients = (0..4)
            .map(|_| {
                let url = url.clone();
                thread::spawn(move || {
                    (0..3).all(|_| {
                        ureq::post(&url)
                            .set("Authorization", &bearer())
                            .send_string(&tool_call("tool"))
                            .is_ok_and(|response| response.status() == 200)
                    })
                })
            })
            .collect::<Vec<_>>();
        assert!(clients.into_iter().all(|client| client.join().unwrap()));
        assert_eq!(most.load(Ordering::SeqCst), 1);
        server.stop();
    }

    #[test]
    fn stop_lets_the_running_call_answer_and_refuses_the_rest() {
        let calls = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicBool::new(false));
        let (started, running) = mpsc::channel();
        let (tool_calls, tool_finished) = (Arc::clone(&calls), Arc::clone(&finished));
        let (server, url) = start(Arc::new(move |_name: &str, _arguments: &Value| {
            tool_calls.fetch_add(1, Ordering::SeqCst);
            let _ = started.send(());
            thread::sleep(Duration::from_millis(150));
            tool_finished.store(true, Ordering::SeqCst);
            Ok(Value::Null)
        }));
        let mut idle = Client::connect(&server);
        idle.send(&sized(&initialize(), ""));
        assert_eq!(idle.answer().status, 200);

        let first_url = url.clone();
        let first = thread::spawn(move || {
            status(
                ureq::post(&first_url)
                    .set("Authorization", &bearer())
                    .send_string(&tool_call("slow")),
            )
        });
        running.recv_timeout(Duration::from_secs(5)).unwrap();
        let queued = thread::spawn(move || {
            status(
                ureq::post(&url)
                    .set("Authorization", &bearer())
                    .send_string(&tool_call("queued")),
            )
        });
        thread::sleep(Duration::from_millis(30));
        server.stop();

        assert!(finished.load(Ordering::SeqCst), "остановка дождалась идущего вызова");
        assert_eq!(first.join().unwrap(), Some(200), "ответ идущего вызова дошёл");
        assert_eq!(queued.join().unwrap(), Some(503), "вызов из очереди отклонён");
        idle.send(&sized(&initialize(), ""));
        let refused = idle.answer();
        assert_eq!((refused.status, refused.connection.as_str()), (503, "close"));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "после остановки инструменты не запускаются");
    }

    #[test]
    fn frees_the_port_after_stop() {
        let (server, url) = server();
        let port = server.port();
        ureq::post(&url)
            .set("Authorization", &bearer())
            .send_string(&initialize())
            .unwrap();
        server.stop();
        assert!(ureq::post(&url).send_string(&initialize()).is_err());
        let caller: SharedToolCaller = Arc::new(|_name: &str, _arguments: &Value| Ok(Value::Null));
        RunningServer::start(port, TOKEN.to_owned(), caller)
            .expect("порт освобождён и сервер поднимается снова")
            .stop();
    }
}
