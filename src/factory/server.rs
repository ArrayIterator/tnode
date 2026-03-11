use crate::use_or_register_factory;
use crate::cores::events::http_events::{HttpRequestEvent, HttpResponseEvent};
use crate::cores::helper::file_info::FileInfo;
use crate::cores::helper::hack::Hack;
use crate::cores::proc::status::Status;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::event_manager::{EventManager, OperationError};
use crate::cores::system::middleware_manager::MiddlewareManager;
use crate::cores::system::routes::Routes;
use crate::cores::system::stats::Stats;
use crate::factory::config::Config;
use crate::factory::factory::Factory;
use crate::factory::server_stats::ServerStats;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServerHandle, Service};
use actix_web::web::Data;
use actix_web::{App, HttpMessage, HttpServer, rt};
use core::clone::Clone;
use core::convert::From;
use log::{debug, info, warn};
use parking_lot::{RwLock};
use rustls::crypto::ring;
use rustls_pki_types::PrivateKeyDer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::sync::broadcast::Receiver;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TcpSocket {
    pub http: Vec<String>,
    pub https: Vec<String>,
}

struct ConnGuard(Arc<AtomicUsize>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct StreamGuardBody {
    inner: BoxBody,
    _ws_guard: ConnGuard, // Guard lo ada di sini
    _req_guard: ConnGuard,
}

impl MessageBody for StreamGuardBody {
    type Error = actix_web::Error;
    fn size(&self) -> actix_web::body::BodySize {
        self.inner.size()
    }

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<actix_web::web::Bytes, Self::Error>>> {
        let inner_pin = Pin::new(&mut self.inner);
        inner_pin
            .poll_next(cx)
            .map_err(|e| actix_web::error::Error::from(e))
    }
}

#[derive(Debug)]
pub struct Server {
    running: Arc<AtomicBool>,
    processing: AtomicBool,
    handle: RwLock<Option<ServerHandle>>,
    start_time: AtomicU64,
    global_requests: Arc<AtomicUsize>,
    total_requests: Arc<AtomicUsize>,
    current_connections: Arc<AtomicUsize>,
    websocket_alive: Arc<AtomicUsize>,
    websocket_requests: Arc<AtomicUsize>,
    total_worker: usize,
    current_config: RwLock<Option<Arc<Config>>>,
    routes: Arc<Routes>,
    service_listener: Arc<EventManager>,
    middleware_manager: Arc<MiddlewareManager>,
    tcp_socket: RwLock<Option<TcpSocket>>,
    start_counter: Arc<AtomicUsize>,
    next_auto_clean: Arc<AtomicIsize>,
}

impl Server {
    pub fn new(routes: Arc<Routes>) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            processing: AtomicBool::new(false),
            handle: RwLock::new(None),
            start_time: AtomicU64::new(0),
            total_requests: Arc::new(AtomicUsize::new(0)),
            current_connections: Arc::new(AtomicUsize::new(0)),
            global_requests: Arc::new(AtomicUsize::new(0)),
            total_worker: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            current_config: RwLock::new(None),
            routes: routes.clone(),
            websocket_alive: Arc::new(AtomicUsize::new(0)),
            websocket_requests: Arc::new(AtomicUsize::new(0)),
            service_listener: use_or_register_factory!(EventManager),
            middleware_manager: use_or_register_factory!(MiddlewareManager),
            tcp_socket: RwLock::new(None),
            start_counter: Arc::new(AtomicUsize::new(0)),
            next_auto_clean: Arc::new(AtomicIsize::new(-1)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn is_processing(&self) -> bool {
        self.processing.load(Ordering::Relaxed)
    }

    pub(crate) fn socket_path_info(config: &Config) -> ResultError<FileInfo> {
        let socket = &config.get_socket()?;
        let info = FileInfo::new(&socket);
        if info.is_exists() {
            if info.is_socket() {
                match UnixStream::connect(socket) {
                    Ok(_) => {
                        return Err(Error::address_in_use(format!(
                            "Socket {} is already in use",
                            socket
                        )));
                    }
                    Err(_) => {
                        // skip stale socket
                        fs::remove_file(socket).map_err(Error::from)?;
                    }
                };
            } else if info.is_file() {
                // check size is zero
                if info.size().unwrap_or_else(|| 0) > 0 {
                    return Err(Error::already_exists(format!(
                        "File {} already exists and is not a socket",
                        socket
                    )));
                }
                fs::remove_file(socket).map_err(Error::from)?
            } else {
                return Err(Error::unsupported(format!(
                    "Path {} already exists and is not a socket",
                    socket
                )));
            }
        }
        Ok(info)
    }
    pub fn start_time(&self) -> u64 {
        self.start_time.load(Ordering::Relaxed)
    }
    pub fn uptime(&self) -> u64 {
        if !self.is_running() {
            return 0;
        }
        let start_time = self.start_time();
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - start_time
    }

    fn reset_stats(&self) {
        self.total_requests.store(0, Ordering::SeqCst);
        self.current_connections.store(0, Ordering::SeqCst);
    }

    //noinspection DuplicatedCode
    pub async fn start(&self, config: Arc<Config>) -> ResultError<()> {
        if self.processing.swap(true, Ordering::SeqCst) {
            return Err(Error::in_progress("Server is already processing start"));
        }
        // prevent multiple start
        if self.running.load(Ordering::SeqCst) {
            return Err(Error::address_in_use("Actix Server is already running"));
        }
        self.reset_stats();
        self.processing.store(false, Ordering::SeqCst);
        let info = Self::socket_path_info(&config).map_err(|e| {
            self.running.store(false, Ordering::SeqCst);
            self.processing.store(false, Ordering::SeqCst);
            e
        })?;
        // get config
        let app = config.app();
        let mut binding: Vec<String> = Vec::new();
        let mut ssl_binding: Vec<String> = Vec::new();
        for b in app.tcp() {
            binding.push(b.to_string());
        }
        let ssl = app.ssl();
        let has_ssl = !ssl.key().is_empty() && !ssl.cert().is_empty() && !ssl.listen().is_empty();
        if has_ssl {
            for b in ssl.listen() {
                ssl_binding.push(b.to_string());
            }
        }
        let sock_path = &info.as_path();
        let socket = &sock_path.to_string_lossy().to_string();
        // make sure the socket directory exists
        if sock_path.parent().is_some() {
            fs::create_dir_all(sock_path.parent().unwrap()).map_err(|e| {
                self.running.store(false, Ordering::SeqCst);
                self.processing.store(false, Ordering::SeqCst);
                Error::permission_denied(format!(
                    "Failed to create socket directory {}: {}",
                    sock_path.parent().unwrap().display(),
                    e
                ))
            })?;
        }

        // Build Actix app
        let middlewares = self.middleware_manager.clone();
        let shared_routes = Data::from(self.get_routes());
        let shared_config = Data::from(config.clone());
        let shared_factory = Data::from(Factory::instance());
        let shared_server = Data::from(Factory::pick_unsafe::<Server>()); // reuse this
        let total_requests = self.total_requests.clone();
        let current_connections = self.current_connections.clone();
        let global_requests = self.global_requests.clone();
        let websocket_requests = self.websocket_requests.clone();
        let websocket_alive = self.websocket_alive.clone();
        let event_listener = self.service_listener.clone();
        let op_timeout = app.operation_timeout_duration();
        let mut server = HttpServer::new(move || {
            let total_req = total_requests.clone();
            let global_req = global_requests.clone();
            let current_conn = current_connections.clone();
            let websocket_req = websocket_requests.clone();
            let current_websocket_conn = websocket_alive.clone();
            // let logger = actix_web::middleware::Logger::default().log_level(Level::Debug);
            let c_event_listener = event_listener.clone();
            // Initialize (LIFO)
            let app = App::new()
                // register dispatcher
                // add shared data
                .app_data(shared_routes.clone()) // add routes
                .app_data(shared_server.clone()) // add shared factory
                .app_data(shared_factory.clone()) // add shared factory
                .app_data(shared_config.clone()) // add shared config (app config)
                .wrap(middlewares.create_dispatcher())
                // Register Logger
                // .wrap(logger)
                // Register outline
                .wrap_fn(move |req, srv| {
                    let r_event_listener = c_event_listener.clone();
                    let http_request_event = HttpRequestEvent::from(&req);
                    let is_websocket_conn = http_request_event.is_websocket();
                    let arc_event = Arc::new(http_request_event.clone());

                    req.extensions_mut().insert(arc_event.clone());

                    let current_active_request = current_conn.clone();
                    let current_active_websocket = current_websocket_conn.clone();
                    // dispatch
                    total_req.fetch_add(1, Ordering::SeqCst);
                    global_req.fetch_add(1, Ordering::SeqCst);
                    current_active_request.fetch_add(1, Ordering::SeqCst);
                    if is_websocket_conn {
                        websocket_req.fetch_add(1, Ordering::SeqCst);
                        current_active_websocket.fetch_add(1, Ordering::SeqCst);
                    }
                    c_event_listener.emit(http_request_event).ok();
                    let fut = srv.call(req);
                    async move {
                        if !is_websocket_conn {
                            let _guard = ConnGuard(current_active_request.clone());
                            let res = tokio::time::timeout(op_timeout, fut).await.unwrap_or_else(
                                |elapsed| {
                                    let err = actix_web::error::ErrorGatewayTimeout(format!(
                                        "Operation Timed Out after: {}",
                                        elapsed
                                    ));
                                    Err(err)
                                },
                            );
                            let http_response_event = HttpResponseEvent::new(arc_event.clone(), &res);
                            r_event_listener.emit(http_response_event).ok();
                            return res;
                        }
                        let mut res = fut.await;
                        let ws_guard = ConnGuard(current_active_websocket);
                        let http_guard = ConnGuard(current_active_request);
                        match res {
                            Ok(srv_res)
                                if srv_res.status()
                                    == actix_web::http::StatusCode::SWITCHING_PROTOCOLS =>
                            {
                                let new_res = srv_res.map_body(move |_head, body| {
                                    BoxBody::new(StreamGuardBody {
                                        inner: body,
                                        _ws_guard: ws_guard,
                                        _req_guard: http_guard,
                                    })
                                });
                                let final_res = Ok(new_res);
                                r_event_listener
                                    .emit(HttpResponseEvent::new(arc_event.clone(), &final_res))
                                    .ok();
                                return final_res;
                            }
                            other => {
                                res = other;
                            }
                        }
                        let http_response_event = HttpResponseEvent::new(arc_event.clone(), &res);
                        r_event_listener.emit(http_response_event).ok();
                        res
                    }
                })
                // Configure routes
                .configure(|service| {
                    shared_routes.conduct(service);
                });
            app
        });

        // 5️⃣ Bind UDS
        let uds = UnixListener::bind(socket).map_err(|e| {
            self.running.store(false, Ordering::SeqCst);
            self.processing.store(false, Ordering::SeqCst);
            Error::address_in_use(format!("Failed to bind to socket {}: {}", socket, e))
        })?;
        let socket_address = config.create_server_socket_address()?;
        let uds_address = UnixListener::bind_addr(&socket_address).map_err(|e| {
            self.running.store(false, Ordering::SeqCst);
            self.processing.store(false, Ordering::SeqCst);
            Error::address_in_use(format!(
                "Failed to bind to file descriptor {:?}: {}",
                socket_address, e
            ))
        })?;
        // try chmod
        match info.chmod(0o660) {
            Ok(_) => {
                debug!(target: "factory", "Socket {} permissions set to 666", socket);
            }
            Err(e) => {
                warn!(target: "factory", "Failed to chmod socket {}: {}", socket, e);
            }
        }

        // set connections
        server = server
            // set connections
            .max_connections(app.max_connections())
            // set rates
            .max_connection_rate(app.max_connections_rate())
            // set rates
            .backlog(app.backlog())
            // set client request timeout
            .client_request_timeout(app.request_timeout_duration())
            // set client disconnect timeout
            .client_disconnect_timeout(app.disconnect_timeout_duration())
            // set keepalive
            .keep_alive(app.keep_alive_duration());
        let workers = app.worker();
        if workers > 0 {
            server = server.workers(workers);
        }

        debug!(target: "factory", "Listening socket: {}", socket);
        server = server.listen_uds(uds).map_err(|e| {
            self.running.store(false, Ordering::SeqCst);
            self.processing.store(false, Ordering::SeqCst);
            Error::permission_denied(format!("Failed to listen on socket {}: {}", socket, e))
        })?;
        debug!(target: "factory", "Listening file descriptor: {:?}", uds_address);
        let uds_address_string = format!("{:?}", uds_address);
        server = server.listen_uds(uds_address).map_err(|e| {
            self.running.store(false, Ordering::SeqCst);
            self.processing.store(false, Ordering::SeqCst);
            Error::permission_denied(format!(
                "Failed to listen on socket {:?}: {}",
                uds_address_string, e
            ))
        })?;

        // 6️⃣ Bind TCP
        for b in &binding {
            server = server.bind(b).map_err(|e| {
                self.running.store(false, Ordering::SeqCst);
                self.processing.store(false, Ordering::SeqCst);
                Error::permission_denied(format!("Failed to listen on {:?}: {}", b, e))
            })?;
        }
        ring::default_provider().install_default().ok();
        let make_ssl_config = || -> ResultError<rustls::ServerConfig> {
            let cert_file = ssl.cert();
            let key_file = ssl.key();
            debug!(target: "factory", "Loading SSL certificate from {}", cert_file);
            let cert = File::open(&cert_file).map_err(|e| {
                Error::invalid_data(format!(
                    "Can not open certificate file for {}: {}",
                    &cert_file, e
                ))
            })?;
            let key = File::open(&key_file).map_err(|e| {
                Error::invalid_data(format!(
                    "Can not open certificate key file for {}: {}",
                    &key_file, e
                ))
            })?;
            let mut cert_reader = BufReader::new(&cert);
            let mut key_reader = BufReader::new(&key);
            let mut certs = Vec::new();
            for c in rustls_pemfile::certs(&mut cert_reader) {
                certs.push(c.map_err(|e| {
                    Error::invalid_data(format!(
                        "Can not load certificate file for {}: {}",
                        cert_file, e
                    ))
                })?);
            }

            debug!(target: "factory", "Loading SSL private key from {}", key_file);
            let key = rustls_pemfile::read_all(&mut key_reader)
                .filter_map(|item| match item {
                    Ok(rustls_pemfile::Item::Pkcs1Key(key)) => Some(PrivateKeyDer::Pkcs1(key)),
                    Ok(rustls_pemfile::Item::Pkcs8Key(key)) => Some(PrivateKeyDer::Pkcs8(key)),
                    Ok(rustls_pemfile::Item::Sec1Key(key)) => Some(PrivateKeyDer::Sec1(key)),
                    _ => None,
                })
                .next()
                .ok_or_else(|| {
                    Error::invalid_data(format!("No private key found in {}", key_file))
                })?;
            let mut config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(|e| {
                    Error::invalid_data(format!("Can not create SSL Server config: {}", e))
                })?;
            config.session_storage =
                rustls::server::ServerSessionMemoryCache::new(ssl.session_cache());
            config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
            Ok(config)
        };

        let ssl_config: rustls::ServerConfig;
        if has_ssl && !ssl_binding.is_empty() {
            debug!(target: "factory", "Binding SSL configuration to addresses: {:?}", ssl_binding);
            ssl_config = make_ssl_config()?;
            for target in &ssl_binding {
                let target_string = format!("{:?}", target);
                server = server
                    .bind_rustls_0_23(target, ssl_config.clone())
                    .map_err(|e| {
                        self.running.store(false, Ordering::SeqCst);
                        self.processing.store(false, Ordering::SeqCst);
                        Error::permission_denied(format!(
                            "Failed to listen on {:?}: {}",
                            target_string, e
                        ))
                    })?;
            }
        }
        info!(target: "factory", "Server is running on {}", binding.join(", "));
        self.start_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::SeqCst,
        );

        // RUN SERVER
        let srv = server.run();
        {
            *self.handle.write() = Some(srv.handle());
            *self.current_config.write() = Some(config.clone());
            *self.tcp_socket.write() = Some(TcpSocket { http: binding, https: ssl_binding });
        }

        self.processing.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);
        let counter = self.start_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let auto_clean_memory_interval = app.auto_clean_memory_interval_duration().as_secs();
        let auto_clean_memory_size = app.auto_clean_memory_size_bytes();
        let total_memory = Stats::total_memory();
        // if enabled & auto clean a memory interval more than 0 & size less than total memory
        let auto_clean = app.auto_clean_memory()
            && auto_clean_memory_interval > 0
            && auto_clean_memory_size < total_memory;
        let atomic_next = self.next_auto_clean.clone();
        atomic_next.store(-1, Ordering::SeqCst);
        if auto_clean {
            let spw_running = self.running.clone();
            let spw_gen_id = self.start_counter.clone();
            let elapsed_atomic = atomic_next.clone();
            // adding malloc trim
            rt::spawn(async move {
                let expo: u64 = 10;
                let loop_ms = 1000 / expo; // 100ms - enough for a fast loop

                let seconds: u64 = auto_clean_memory_interval;
                let mut i = tokio::time::interval(Duration::from_millis(loop_ms));
                let counter_data = seconds * expo;
                let safe_memory = auto_clean_memory_size;
                let mut seconds_counter = 0;
                info!(
                    "Spawning memory cleaner on server by counter: {} every {} with memory reached by: {}",
                    counter,
                    Hack::format_duration(Duration::from_secs(seconds), false),
                    Hack::format_size_with_precision(auto_clean_memory_size, 2, false)
                );
                elapsed_atomic.store(seconds as isize, Ordering::SeqCst);
                loop {
                    i.tick().await;
                    if !spw_running.load(Ordering::SeqCst)
                        || spw_gen_id.load(Ordering::SeqCst) != counter
                    {
                        break;
                    }
                    seconds_counter += 1;
                    let elapsed_seconds = (counter_data - seconds_counter) / expo;
                    elapsed_atomic.store(elapsed_seconds as isize, Ordering::SeqCst);
                    if seconds_counter >= counter_data {
                        let memory_usage = Status::get().memory.vm_rss;
                        if safe_memory > memory_usage {
                            debug!(
                                "Skipping memory cleaner for : {} that minimum is : {}",
                                Hack::format_size_trim(memory_usage),
                                Hack::format_size_trim(safe_memory)
                            );
                            seconds_counter = 0; // Reset counter
                            elapsed_atomic.store(seconds as isize, Ordering::SeqCst);
                            continue;
                        }
                        #[cfg(target_os = "linux")]
                        unsafe {
                            elapsed_atomic.store(0, Ordering::SeqCst);
                            info!(target: "factory", "Cleaning up memory on server by counter: {}", counter);
                            let time = Instant::now();
                            nix::libc::malloc_trim(0); // clean up
                            let elapsed = Instant::now() - time;
                            let elapsed = Hack::format_duration(elapsed, false);
                            Status::refresh();
                            let old_memory = Hack::format_size_trim(memory_usage);
                            let memory_usage = Status::get().memory.vm_rss;
                            info!(
                                "Cleaned up memory on server by counter: {}, from: {} to : {} for : {}",
                                counter,
                                old_memory,
                                Hack::format_size_trim(memory_usage),
                                elapsed
                            );
                        }
                        elapsed_atomic.store(seconds as isize, Ordering::SeqCst);
                        seconds_counter = 0; // Reset counter
                    }
                }
                atomic_next.store(0, Ordering::SeqCst);
                info!(target: "factory", "Memory cleaner stopped on server counter: {}", counter);
            });
        }

        // clone state
        if let Err(e) = srv.await {
            warn!(target: "factory", "server stopped: {}", e);
        }
        // WRITE LOCKS - Mung sedilut pas setup
        {
            *self.handle.write() = None;
            *self.current_config.write() = None;
            *self.tcp_socket.write() = None;
        }
        self.running.store(false, Ordering::SeqCst);
        self.reset_stats(); // reset the stats
        Ok(())
    }

    pub async fn stop(&self) {
        self.stop_action(false).await;
    }

    pub async fn stop_action(&self, graceful: bool) {
        let handle = self.handle.write().take();
        self.tcp_socket.write().take();
        if let Some(h) = handle {
            h.stop(graceful).await;
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        if let Some(conf) = self.current_config.write().take() {
            let _ = fs::remove_file(conf.app().socket());
        }
        self.reset_stats();
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn subscribe_request(&self) -> OperationError<Receiver<HttpRequestEvent>> {
        self.service_listener.subscribe::<HttpRequestEvent>()
    }
    pub fn subscribe_response(&self) -> OperationError<Receiver<HttpResponseEvent>> {
        self.service_listener.subscribe::<HttpResponseEvent>()
    }

    pub fn get_tcp_socket(&self) -> Option<TcpSocket> {
        self.tcp_socket.read().clone()
    }

    pub async fn restart(&self, config: Arc<Config>) -> ResultError<()> {
        self.stop().await;
        info!(target: "factory", "Restarting server...");
        self.start(config).await
    }
    pub async fn restart_gracefully(
        &self,
        config: Arc<Config>,
        gracefully: bool,
    ) -> ResultError<()> {
        self.stop_action(gracefully).await;
        info!(target: "factory", "Restarting server...");
        self.start(config).await
    }
    pub fn get_routes(&self) -> Arc<Routes> {
        self.routes.clone()
    }
    pub fn get_total_worker(&self) -> usize {
        self.get_current_config().map(|e|e.app().worker()).unwrap_or(self.total_worker)
    }
    pub fn get_total_requests(&self) -> usize {
        self.total_requests.load(Ordering::SeqCst)
    }
    pub fn get_global_requests(&self) -> usize {
        self.global_requests.load(Ordering::SeqCst)
    }
    pub fn get_active_connections(&self) -> usize {
        self.current_connections.load(Ordering::SeqCst)
    }
    pub fn get_websocket_request(&self) -> usize {
        self.websocket_requests.load(Ordering::SeqCst)
    }
    pub fn get_active_websocket(&self) -> usize {
        self.websocket_alive.load(Ordering::SeqCst)
    }
    pub fn get_start_counter(&self) -> usize {
        self.start_counter.load(Ordering::SeqCst)
    }
    pub fn get_uptime(&self) -> u64 {
        self.uptime()
    }
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.total_requests.load(Ordering::SeqCst),
            self.current_connections.load(Ordering::SeqCst),
        )
    }

    pub fn get_next_auto_clean(&self) -> isize {
        self.next_auto_clean.load(Ordering::SeqCst)
    }

    pub fn get_current_config(&self) -> Option<Arc<Config>> {
        self.current_config.read().clone()
    }
    pub fn server_info(&self) -> ServerStats {
        ServerStats::from_server(self)
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new(Factory::pick_unsafe::<Routes>())
    }
}
