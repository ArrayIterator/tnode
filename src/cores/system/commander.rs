use crate::cores::system::error::{Error, ErrorType, ResultError};
use crate::cores::system::runtime::Runtime;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::net::{SocketAddr as SSock, UnixDatagram as UdGram};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::unix::SocketAddr;
use tokio::net::UnixDatagram;

pub trait Formatter: Debug + Send + Sync + 'static {
    /// Formats a command along with its ID and an optional message into a JSON string.
    ///
    /// # Parameters
    /// - `command`: A string slice representing the command to be formatted.
    /// - `id`: A string slice representing the unique identifier associated with the command.
    /// - `message`: An optional `String` containing a message to be included in the formatted JSON.
    ///
    /// # Returns
    /// - On success, returns a `ResultOk<String>` containing the serialized JSON string representation of the command.
    /// - On error, returns a `ResultError<String>` encapsulating an error when JSON serialization fails.
    ///
    /// # Errors
    /// - Returns an `Error::invalid_data` if the serialization process fails, which includes any serialization error details in the error message.
    ///
    /// # Example
    /// ```rust
    /// let result = instance.format("start", "123", Some("Initialization successful".to_string()));
    /// match result {
    ///     Ok(json) => println!("Formatted JSON: {}", json),
    ///     Err(err) => eprintln!("Error: {}", err),
    /// }
    /// ```
    ///
    /// This function is typically used to prepare a structured JSON-based command payload.
    fn format(&self, command: &str, id: &str, message: Option<String>) -> ResultError<String> {
        let command = String::from(command);
        let id = String::from(id);
        serde_json::to_string(&serde_json::json!(ControlCommand {
            command,
            id,
            message
        }))
        .map_err(|e| Error::invalid_data(format!("Failed to format command: {}", e)))
    }
}

pub trait ParserInto {
    /// Parses a byte buffer into a deserializable type `T`.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `Serialize`, `Deserialize`, and `'static` traits.
    ///
    /// # Parameters
    /// - `buff`: A byte slice (`&[u8]`) containing the data to be parsed.
    /// - `size`: The size of the intended portion of `buff` to parse (only the first `size` bytes
    ///   of the buffer will be used).
    ///
    /// # Returns
    /// - `ResultError<T>`:
    ///   - On success, returns the parsed value of type `T`.
    ///   - On failure, returns an `Error` derived from the deserialization process.
    ///
    /// # Constraints
    /// - Requires the caller type (`Self`) to implement the `Sized` trait.
    ///
    /// # Errors
    /// - This function will return an error if:
    ///   1. The given byte buffer cannot be converted to a UTF-8 string.
    ///   2. The string cannot be deserialized as type `T`.
    ///
    /// # Notes
    /// - The method slices the buffer to encompass only the first `size` bytes.
    /// - It utilizes `serde_json` for deserialization assuming the input data
    ///   is a JSON representation.
    ///
    /// # Example
    /// ```rust
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Example {
    ///     value: String,
    /// }
    ///
    /// // Assuming `parse_into` is a method in a trait implemented for `MyStruct`.
    /// let my_struct = MyStruct {};
    /// let buffer: &[u8] = br#"{"value": "Hello, world!"}"#;
    /// let result: Result<Example, Error> = my_struct.parse_into(buffer, buffer.len());
    ///
    /// match result {
    ///     Ok(parsed) => println!("{:?}", parsed),
    ///     Err(e) => eprintln!("Failed to parse buffer: {:?}", e),
    /// }
    /// ```
    fn parse_into<T: Serialize + for<'de> Deserialize<'de> + 'static>(
        &self,
        buff: &[u8],
        size: usize,
    ) -> ResultError<T>
    where
        Self: Sized,
    {
        let buff = &buff[0..size];
        serde_json::from_str::<T>(String::from_utf8_lossy(buff).as_ref()).map_err(Error::from_error)
    }
}

#[derive(Debug, Clone)]
pub struct JsonFormatter;
impl Formatter for JsonFormatter {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ControlCommand {
    pub command: String,
    pub id: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ControlResponder {
    pub command: String,
    pub id: String,
    pub status: u32,
    pub message: String,
}

impl Default for ControlCommander {
    fn default() -> Self {
        Self::new(Box::new(JsonFormatter))
    }
}

#[derive(Debug)]
pub struct Datagram {
    datagram: Arc<UnixDatagram>,
}
impl ParserInto for Datagram {}
impl Datagram {
    pub fn new(unix_datagram: Arc<UnixDatagram>) -> Self {
        Self {
            datagram: unix_datagram,
        }
    }
    pub async fn send_to(&self, target: &SocketAddr, buf: &[u8]) -> ResultError<usize> {
        if let Some(path) = target.as_pathname() {
            self.datagram.send_to(buf, path).await.map_err(Error::from)
        } else {
            let std_addr: SSock = target.clone().into();
            let std_sock =
                unsafe { ManuallyDrop::new(UdGram::from_raw_fd(self.datagram.as_raw_fd())) };
            std_sock.send_to_addr(buf, &std_addr).map_err(Error::from)
        }
    }
    pub async fn send_message_to<Message: AsRef<str>>(
        &self,
        target: &SocketAddr,
        data: Message,
    ) -> ResultError<usize> {
        self.send_to(target, data.as_ref().as_bytes())
            .await
            .map_err(|e| e)
    }
    pub async fn send_object_to<Object: 'static + Send + Sync + Serialize>(
        &self,
        target: &SocketAddr,
        data: Object,
    ) -> ResultError<usize> {
        self.send_to(
            target,
            serde_json::to_string(&serde_json::json!(data))
                .map_err(|e| Error::invalid_data(format!("Failed to format command: {}", e)))?
                .as_bytes(),
        )
        .await
        .map_err(|e| e)
    }
    pub fn datagram(&self) -> &UnixDatagram {
        &self.datagram
    }
}

impl Deref for Datagram {
    type Target = Arc<UnixDatagram>;
    fn deref(&self) -> &Self::Target {
        &self.datagram
    }
}

#[derive(Debug)]
pub struct ControlCommander {
    socket: Mutex<Option<Arc<UnixDatagram>>>,
    formatter: Box<dyn Formatter>,
}
impl ParserInto for ControlCommander {}
impl ControlCommander {
    /// Creates a new instance of the struct.
    ///
    /// # Arguments
    ///
    /// * `formatter` - A boxed object implementing the `Formatter` trait, used to format data within the struct.
    ///
    /// # Returns
    ///
    /// A new instance of the struct with:
    /// - `socket` initialized as a `Mutex` containing `None`.
    /// - The provided `formatter` assigned to the struct.
    ///
    /// # Example
    /// ```rust
    /// let my_formatter: Box<dyn Formatter> = Box::new(MyFormatter::new());
    /// let instance = MyStruct::new(my_formatter);
    /// ```
    pub fn new(formatter: Box<dyn Formatter>) -> Self {
        Self {
            socket: Mutex::new(None),
            formatter,
        }
    }

    /// Sets the formatter for the current instance.
    ///
    /// This method allows you to provide a custom implementation of the
    /// `Formatter` trait, which will be used internally by the instance
    /// for formatting operations.
    ///
    /// # Parameters
    /// - `formatter`: A boxed trait object implementing the `Formatter`
    ///   trait. This allows for runtime polymorphism, enabling the use
    ///   of different formatting strategies.
    ///
    /// # Examples
    /// ```rust
    /// struct CustomFormatter;
    /// impl Formatter for CustomFormatter {
    ///     // Implementation of the required methods
    /// }
    ///
    /// let mut instance = MyStruct::new();
    /// instance.set_formatter(Box::new(CustomFormatter));
    /// ```
    ///
    /// In the example above, the `CustomFormatter` implementation of
    /// the `Formatter` trait is boxed and passed to the `set_formatter`
    /// method, replacing the current formatter.
    ///
    /// # Note
    /// This method replaces any existing formatter previously set for the instance.
    pub fn set_formatter(&mut self, formatter: Box<dyn Formatter>) {
        self.formatter = formatter;
    }

    /// Provides a reference to the `Formatter` instance associated with the current object.
    ///
    /// This method returns a reference to a boxed trait object that implements the `Formatter` trait.
    ///
    /// # Returns
    ///
    /// A reference to the `Box<dyn Formatter>` that is associated with this instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let formatter = object.formatter();
    /// formatter.format(...); // Use the Formatter's functionality here
    /// ```
    pub fn formatter(&self) -> &Box<dyn Formatter> {
        &self.formatter
    }

    /// Closes the socket by taking ownership of the underlying `socket` object.
    ///
    /// This function acquires a lock on the `socket` field, which is expected to be wrapped in
    /// a `Mutex<Option<T>>`. It then replaces the `Option` with `None`, effectively taking ownership
    /// of the socket and releasing any resources associated with it.
    ///
    /// # Notes
    /// - If the `socket` was already `None` when this method is called, this operation has no effect.
    /// - This operation is thread-safe due to the use of a `Mutex`.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Mutex;
    ///
    /// struct MyStruct {
    ///     socket: Mutex<Option<String>>, // Example socket type
    /// }
    ///
    /// impl MyStruct {
    ///     pub fn close(&self) {
    ///         self.socket.lock().unwrap().take();
    ///     }
    /// }
    ///
    /// let instance = MyStruct {
    ///     socket: Mutex::new(Some("SocketResource".to_string())),
    /// };
    /// instance.close();
    /// assert!(instance.socket.lock().unwrap().is_none());
    /// ```
    pub fn close(&self) {
        self.socket.lock().take();
    }

    /// Checks and retrieves a clone of the `UnixDatagram` socket if it exists.
    ///
    /// This function attempts to acquire a lock on the `socket` field,
    /// which is expected to be an `Option` containing a thread-safe reference
    /// to a `UnixDatagram`. If the `socket` is `None`, the function returns `None`.
    /// If the `socket` contains a value, it clones the contained `Arc<UnixDatagram>`
    /// and returns it.
    ///
    /// # Returns
    /// * `Option<Arc<UnixDatagram>>`:
    ///     - `Some(Arc<UnixDatagram>)` if the socket exists.
    ///     - `None` if the socket is not set.
    ///
    /// # Example
    /// ```rust
    /// if let Some(socket) = instance.socket_check() {
    ///     // Use the socket instance as needed
    /// } else {
    ///     // Handle the case where the socket is not available
    /// }
    /// ```
    ///
    /// # Thread Safety
    /// This method uses a lock to ensure thread-safe access to the `socket` field.
    pub fn socket_check(&self) -> Option<Arc<UnixDatagram>> {
        match self.socket.lock().as_ref() {
            None => None,
            Some(e) => Some(e.clone()),
        }
    }

    /// Attempts to establish a connection by creating and configuring a Unix domain datagram socket.
    ///
    /// # Details
    /// This method ensures that a unique local address is generated for the client, binds the socket
    /// to the local address, and establishes a connection to the target server address. Additionally,
    /// it sets the socket to nonblocking mode and caches the created socket in an `Arc` for reuse
    /// during subsequent calls.
    ///
    /// # Steps:
    /// 1. If a socket already exists in the internal cache (protected by a mutex), it is cloned and returned.
    /// 2. Otherwise:
    ///    - A unique client address is generated using [`Runtime::datagram_client_unique_address`].
    ///    - The target server address is retrieved using [`Runtime::datagram_server_address`].
    ///    - A datagram socket is created and bound to the generated client address.
    ///    - The socket is configured to nonblocking mode.
    ///    - The socket is connected to the server address.
    ///    - The socket is wrapped in an `Arc`, stored in the internal cache, and returned.
    ///
    /// # Errors
    /// This method can return an error in the following cases:
    /// - Failure to retrieve a unique client address or target server address.
    /// - Failure to bind the socket to the client address.
    /// - Failure to set the socket into nonblocking mode.
    /// - Failure to connect to the target server address.
    /// - Failure to create a Unix domain socket from the underlying datagram socket.
    ///
    /// # Returns
    /// - `Ok(Arc<UnixDatagram>)`: If the connection succeeds, a reference-counted Unix domain datagram socket is returned.
    /// - `Err(Error)`: If any step during the connection establishment process fails, an appropriate error is returned.
    ///
    /// # Example
    /// ```rust
    /// let connection = my_object.connect();
    /// match connection {
    ///     Ok(socket) => {
    ///         println!("Socket successfully connected!");
    ///         // Use `socket` for communication.
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to connect: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Thread-Safety
    /// This method is thread-safe due to the use of a mutex-protected cache for storing the socket.
    ///
    /// # Important Notes
    /// - The connection established is based on the Unix domain sockets and hence only works in environments
    ///   where such sockets are supported.
    /// - If the cache already contains a socket, it is reused for reducing overhead.
    pub fn connect(&self) -> ResultError<Arc<UnixDatagram>> {
        if let Some(socket) = self.socket.lock().as_ref() {
            return Ok(socket.clone());
        }
        let datagram_client = Runtime::datagram_client_unique_address()?;
        let target_address = Runtime::datagram_server_address()?;
        let sync_socket = UdGram::bind_addr(&datagram_client)
            .map_err(|e| Error::address_not_available(format!("Failed to bind client: {}", e)))?;
        sync_socket.set_nonblocking(true).map_err(|e| {
            Error::other(format!(
                "Failed to set nonblocking on control socket {:?}: {}",
                datagram_client, e
            ))
        })?;
        sync_socket.connect_addr(&target_address).map_err(|e| {
            Error::address_not_available(format!(
                "Failed to control server {:?} : {}",
                target_address, e
            ))
        })?;
        let socket = UnixDatagram::from_std(sync_socket)
            .map_err(|e| Error::address_not_available(format!("Failed to create socket: {}", e)))?;

        let socket = Arc::new(socket);
        {
            let mut sock = self.socket.lock();
            *sock = Some(socket.clone());
        }
        Ok(socket)
    }

    /// Executes a looping asynchronous operation using a provided closure.
    ///
    /// This function spawns a task that runs the provided asynchronous closure in a loop
    /// until a termination condition is met. It manages the lifetime of the socket, the stopping
    /// signal, and ensures that the closure is executed concurrently in a predictable way.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the closure that defines the asynchronous task to execute in the loop.
    /// - `Fut`: The type of the `Future` returned by the closure, with an output of `()`.
    ///
    /// # Parameters
    ///
    /// - `self`: Consumes the instance of `Self` and provides it to the asynchronous operation.
    /// - `func`: A closure that takes three arguments:
    ///     - `Arc<UnixDatagram>`: A reference to the datagram socket.
    ///     - `Arc<Self>`: A reference to the current instance of `Self`.
    ///     - `Arc<AtomicBool>`: A flag indicating whether the task should stop.
    ///
    /// # Returns
    ///
    /// A `Result` that contains `()` on success, or an error of type `ResultError<()>` on failure.
    ///
    /// # Behavior
    ///
    /// 1. Creates a stopping signal (`arc_stop`) and initializes it to `false`.
    /// 2. Establishes a shared `Arc` reference (`this`) to the current instance.
    /// 3. Attempts to connect the socket using `self.connect()`; returns an error if unsuccessful.
    /// 4. Executes the `func` closure in a loop:
    ///     - When `arc_stop` is set to `true`, or when the socket is released (i.e., no longer available),
    ///       the loop terminates.
    ///     - Spawns the closure into a `tokio` task, running it asynchronously.
    ///     - Monitors the `done` flag to determine if the spawned task has completed and rechecks
    ///       stopping conditions periodically (100ms intervals).
    ///
    /// # Stopping Conditions
    ///
    /// The loop stops when any of the following conditions are met:
    /// - The `arc_stop` flag is set to `true`.
    /// - The `done` flag for the spawned asynchronous task is set to `true`.
    /// - The socket is no longer available.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    /// use tokio::net::UnixDatagram;
    /// use futures::future::Future;
    ///
    /// async fn example_function(socket: Arc<UnixDatagram>, instance: Arc<Self>, stop_flag: Arc<AtomicBool>) {
    ///     // Perform some asynchronous operation here.
    /// }
    ///
    /// let result = some_instance.looping(example_function).await;
    ///
    /// if let Err(err) = result {
    ///     eprintln!("Error: {:?}", err);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// The function will return an error of type `ResultError<()>` if the connection to the socket
    /// cannot be established (`self.connect()` fails).
    ///
    /// # Notes
    ///
    /// - This method relies on `Arc` references to manage shared state between tasks.
    /// - Task completion is tracked using atomic flags (`AtomicBool`) to ensure proper synchronization across threads.
    /// - The function relies on the `tokio` runtime for scheduling asynchronous tasks and timeouts.
    pub async fn looping<F, Fut>(self, func: F) -> ResultError<()>
    where
        F: Fn(Arc<UnixDatagram>, Arc<Self>, Arc<AtomicBool>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + 'static + Send,
    {
        let arc_stop = Arc::new(AtomicBool::new(false));
        let this = Arc::new(self);
        let refs = Arc::clone(&this);
        let socket = refs.connect()?;
        let arc_fn = Arc::new(func);
        loop {
            if arc_stop.load(Ordering::Relaxed) {
                break;
            }
            let has_socket = {
                let guard = this.socket.lock();
                guard.is_some()
            };
            if !has_socket {
                break;
            }
            let done = Arc::new(AtomicBool::new(false));
            let borrow_done = Arc::clone(&done);
            let fn_clone = Arc::clone(&arc_fn);
            let arc_clone = Arc::clone(&arc_stop);
            let socket_clone = Arc::clone(&socket);
            let async_fn = |s, b, m| async move {
                fn_clone(s, b, m).await;
                borrow_done.store(true, Ordering::Relaxed);
            };
            let inner = async_fn(socket_clone, this.clone(), arc_clone);
            tokio::spawn(async move {
                inner.await;
                Ok::<(), Error>(())
            });
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if done.load(Ordering::Relaxed) {
                    break;
                }
                if arc_stop.load(Ordering::Relaxed) {
                    break;
                }
                let has_socket = {
                    arc_stop.store(true, Ordering::Relaxed);
                    let guard = this.socket.lock();
                    guard.is_some()
                };
                if !has_socket {
                    arc_stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Asynchronously sends a command to the control socket associated with the instance.
    ///
    /// # Parameters
    /// - `command`: A generic parameter implementing `AsRef<str>`, representing the primary command to be sent.
    /// - `id`: A generic parameter implementing `AsRef<str>`, representing the unique identifier associated with the command.
    /// - `message`: An optional `String` containing an additional message or payload to accompany the command.
    ///
    /// # Returns
    /// - On success, returns a `ResultError<Arc<UnixDatagram>>` containing the control socket wrapped in an `Arc`.
    /// - On failure, returns an error of type `ResultError` with details about the encountered issue.
    ///
    /// # Errors
    /// This function may return an error in the following cases:
    /// - If it fails to connect to the control socket, with specific handling for the following error types:
    ///   - `ErrorType::PermissionDenied`
    ///   - `ErrorType::ConnectionRefused`
    ///   - `ErrorType::ConnectionReset`
    ///   - `ErrorType::HostUnreachable`
    ///   - `ErrorType::NetworkUnreachable`
    ///   - `ErrorType::ConnectionAborted`
    ///   - `ErrorType::NotConnected`
    ///   If any of these errors occur, the control socket is closed, and an appropriate error message is returned.
    /// - If the command fails to format using the formatter provided by `self.formatter()`.
    /// - If the socket fails to send the command, with an error message detailing the failure.
    ///
    /// # Usage
    /// ```rust
    /// let result = my_instance
    ///     .send("COMMAND_NAME", "command_id", Some("Optional message"))
    ///     .await;
    ///
    /// match result {
    ///     Ok(socket) => println!("Command sent successfully"),
    ///     Err(e) => eprintln!("Error occurred: {}", e),
    /// }
    /// ```
    ///
    /// # Notes
    /// - The function internally manages reconnection attempts by closing the control socket upon certain types of errors.
    /// - Operations are performed asynchronously and will require an async runtime for execution.
    ///
    /// # Example Error Handling
    /// ```rust
    /// if let Err(e) = instance.send("RESTART", "12345", None).await {
    ///     match e.kind() {
    ///         ErrorType::PermissionDenied => println!("Permission denied: {}", e),
    ///         ErrorType::ConnectionRefused => println!("Connection refused: {}", e),
    ///         _ => eprintln!("Unhandled error: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn send<T: AsRef<str>, Id: AsRef<str>>(
        &self,
        command: T,
        id: Id,
        message: Option<String>,
    ) -> ResultError<Arc<UnixDatagram>> {
        match self.connect() {
            Err(e) => {
                match e.kind() {
                    ErrorType::PermissionDenied
                    | ErrorType::ConnectionRefused
                    | ErrorType::ConnectionReset
                    | ErrorType::HostUnreachable
                    | ErrorType::NetworkUnreachable
                    | ErrorType::ConnectionAborted
                    | ErrorType::NotConnected => self.close(),
                    _ => {}
                }
                Err(e)
            }
            Ok(socket) => {
                let command = self
                    .formatter()
                    .format(command.as_ref(), id.as_ref(), message)?;
                socket.send(command.as_bytes()).await.map_err(|e| {
                    Error::interrupted(format!(
                        "Failed to send info command to control socket: {}",
                        e
                    ))
                })?;
                Ok(socket)
            }
        }
    }

    /// Sends a command with an optional message, waits for a response, and returns the result.
    ///
    /// # Type Parameters
    /// - `T`: A type that can be referenced as a string slice (`&str`) for the `command`.
    /// - `Id`: A type that can be referenced as a string slice (`&str`) for the `id`.
    ///
    /// # Arguments
    /// - `command` - The command to be sent, provided as a string-like object.
    /// - `id` - Identifier associated with the command, provided as a string-like object.
    /// - `message` - An optional string message to send along with the command.
    ///
    /// # Returns
    /// - On success, returns a `ResultError` containing a tuple with:
    ///   - A `Vec<u8>` representing the received data.
    ///   - A `usize` with the number of bytes received.
    ///   - A `SocketAddr` identifying the address of the sender.
    ///   - An `Arc<UnixDatagram>` representing the connection used.
    /// - On failure, returns an error encapsulated in a `ResultError`.
    ///
    /// # Behavior
    /// This function delegates to `send_receive_timeout` with a default timeout of 5 seconds.
    /// It is an asynchronous function and must be awaited.
    ///
    /// # Errors
    /// Returns an error if the operation fails, such as in cases of timeout, network issues, or
    /// internal errors.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    /// use tokio::net::UnixDatagram;
    /// use std::net::SocketAddr;
    ///
    /// async fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    ///     let handler = MyHandler::new(); // Assuming `MyHandler` is your struct with this method.
    ///
    ///     let response = handler
    ///         .send_receive("COMMAND", "1234", Some("optional message".to_string()))
    ///         .await?;
    ///
    ///     let (data, size, address, connection) = response;
    ///     println!("Received {} bytes from {}: {:?}", size, address, data);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send_receive<T: AsRef<str>, Id: AsRef<str>>(
        &self,
        command: T,
        id: Id,
        message: Option<String>,
    ) -> ResultError<(Vec<u8>, usize, SocketAddr, Arc<UnixDatagram>)> {
        self.send_receive_timeout(command, id, message, std::time::Duration::from_secs(5))
            .await
    }

    /// Sends a command with an optional message and waits for a response from a Unix datagram socket
    /// with a specified timeout duration.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements `AsRef<str>`, representing the command to be sent.
    /// - `Id`: A type that implements `AsRef<str>`, representing the identifier associated with the message.
    ///
    /// # Parameters
    /// - `command`: The command to be sent to the socket.
    /// - `id`: The identifier for the command.
    /// - `message`: An optional message to be sent along with the command. Can be `None` if no message is required.
    /// - `timeout`: The maximum duration to wait for a response from the socket.
    ///
    /// # Returns
    /// - `Ok((Vec<u8>, usize, SocketAddr, Arc<UnixDatagram>))`: A tuple containing:
    ///   - `Vec<u8>`: The received data as a vector of bytes.
    ///   - `usize`: The size of the received data.
    ///   - `SocketAddr`: The address of the socket from which the data was received.
    ///   - `Arc<UnixDatagram>`: A reference to the Unix datagram socket used for communication.
    /// - `Err(Error)`: An error if sending, receiving, or timeout operations fail.
    ///
    /// # Errors
    /// - Returns a timeout error if the response is not received within the specified duration.
    /// - Returns an error if the `recv_from` call fails due to any other reason.
    ///
    /// # Constraints
    /// - The response data has a maximum size limit of 65535 bytes (64KB), which is the limit for a datagram socket.
    ///
    /// # Example
    /// ```rust
    /// let result = my_socket.send_receive_timeout(
    ///     "my_command",
    ///     "123",
    ///     Some("payload".to_string()),
    ///     std::time::Duration::from_secs(5)
    /// ).await;
    ///
    /// match result {
    ///     Ok((data, size, addr, socket)) => {
    ///         println!("Received {} bytes from {:?}: {:?}", size, addr, data);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to communicate: {}", e);
    ///     }
    /// }
    /// ```
    pub async fn send_receive_timeout<T: AsRef<str>, Id: AsRef<str>>(
        &self,
        command: T,
        id: Id,
        message: Option<String>,
        timeout: std::time::Duration,
    ) -> ResultError<(Vec<u8>, usize, SocketAddr, Arc<UnixDatagram>)> {
        let socket = self.send(command, id, message).await?;
        // 64KB limit of Datagram socket
        let mut buff = vec![0u8; 65535];
        self.receive_timeout(timeout, Some(socket)).await
    }

    pub async fn receive_timeout(
        &self,
        timeout: std::time::Duration,
        socket_dgram: Option<Arc<UnixDatagram>>,
    ) -> ResultError<(Vec<u8>, usize, SocketAddr, Arc<UnixDatagram>)> {
        let socket = if let Some(arc) = socket_dgram {
            arc
        } else {
            self.connect()?
        };
        let mut buff = vec![0u8; 65535];
        let (size, sock) = tokio::time::timeout(timeout, socket.recv_from(&mut buff))
            .await
            .map_err(|_| {
                Error::timeout(format!(
                    "Timeout waiting for response from socket {:?}",
                    socket
                ))
            })?
            .map_err(|e| Error::other(format!("Recv error: {}", e)))?;
        let final_data = buff[..size].to_vec();
        Ok((final_data, size, sock, socket))
    }
}
