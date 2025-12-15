# Go-Rust Interoperability Guide

This guide covers the various methods for Go code to communicate with Rust code, ranked by latency and optimization potential.

## Overview

When building systems that combine Go and Rust, choosing the right interoperability method is critical for performance. This guide ranks approaches from lowest to highest latency.

---

## Methods Ranked by Latency

| Rank | Method              | Latency    | Throughput  | Complexity | Use Case                         |
| ---- | ------------------- | ---------- | ----------- | ---------- | -------------------------------- |
| 1    | CGo + FFI           | ~10-50ns   | Highest     | High       | In-process, performance-critical |
| 2    | Shared Memory       | ~100-500ns | Very High   | High       | Large data transfers             |
| 3    | Unix Domain Sockets | ~1-10μs    | High        | Medium     | Local IPC                        |
| 4    | gRPC (UDS)          | ~10-50μs   | High        | Low        | Structured local RPC             |
| 5    | gRPC (TCP)          | ~50-200μs  | Medium-High | Low        | Network RPC                      |
| 6    | HTTP/REST           | ~100-500μs | Medium      | Low        | Simple APIs                      |
| 7    | Message Queues      | ~100μs-1ms | Medium      | Medium     | Async, decoupled systems         |
| 8    | File-based IPC      | ~1-10ms    | Low         | Low        | Simple, batch processing         |

### Serialization Formats for FFI

When using CGo + FFI or Shared Memory, the choice of serialization format significantly impacts performance:

| Format      | Ser/Deser Time | Zero-Copy | Schema | Best For                    |
| ----------- | -------------- | --------- | ------ | --------------------------- |
| Raw structs | 0 (direct)     | ✅         | Manual | Simple, fixed-size data     |
| Cap'n Proto | ~10-50ns       | ✅         | Yes    | Complex data, max perf      |
| FlatBuffers | ~20-100ns      | ✅         | Yes    | Gaming, mobile, embedded    |
| Protobuf    | ~100ns-1μs     | ❌         | Yes    | General RPC, good ecosystem |
| JSON        | ~1-10μs        | ❌         | No     | Debugging, simplicity       |
| MessagePack | ~500ns-2μs     | ❌         | No     | Compact JSON alternative    |

---

## 1. CGo + FFI (Foreign Function Interface)

**Latency: ~10-50 nanoseconds per call**

The fastest method—Rust compiles to a C-compatible shared library that Go calls directly via CGo.

### How It Works

```
┌─────────────┐     C ABI      ┌─────────────┐
│   Go Code   │◄──────────────►│  Rust .so   │
│   (CGo)     │   Direct Call  │  (cdylib)   │
└─────────────┘                └─────────────┘
```

### Rust Side

```rust
// lib.rs
#[no_mangle]
pub extern "C" fn add_numbers(a: i64, b: i64) -> i64 {
    a + b
}

#[no_mangle]
pub extern "C" fn process_buffer(ptr: *const u8, len: usize) -> i64 {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    slice.iter().map(|&x| x as i64).sum()
}

// For strings - caller must free
#[no_mangle]
pub extern "C" fn get_greeting(name: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(name) };
    let name = c_str.to_str().unwrap_or("World");
    let greeting = format!("Hello, {}!", name);
    CString::new(greeting).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}
```

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]
```

### Go Side

```go
package main

/*
#cgo LDFLAGS: -L./target/release -lmyrust -ldl -lm
#include <stdlib.h>

extern long long add_numbers(long long a, long long b);
extern long long process_buffer(const unsigned char* ptr, size_t len);
extern char* get_greeting(const char* name);
extern void free_string(char* s);
*/
import "C"
import "unsafe"

func AddNumbers(a, b int64) int64 {
    return int64(C.add_numbers(C.longlong(a), C.longlong(b)))
}

func ProcessBuffer(data []byte) int64 {
    if len(data) == 0 {
        return 0
    }
    return int64(C.process_buffer((*C.uchar)(&data[0]), C.size_t(len(data))))
}

func GetGreeting(name string) string {
    cName := C.CString(name)
    defer C.free(unsafe.Pointer(cName))
    
    result := C.get_greeting(cName)
    defer C.free_string(result)
    
    return C.GoString(result)
}
```

### Pros & Cons

| Pros                      | Cons                                  |
| ------------------------- | ------------------------------------- |
| Lowest latency            | Complex memory management             |
| No serialization overhead | CGo has ~50ns call overhead           |
| Direct memory access      | Harder to debug                       |
| Type-safe at compile time | Platform-specific builds              |
| No network stack          | Cannot use goroutines across boundary |

### When to Use

- Cryptographic operations
- Hot paths called millions of times
- SIMD/vectorized computations
- When you need zero-copy data access

### Serialization Considerations for FFI

When passing complex data across FFI boundaries, you have several options:

#### Option 1: Raw C Structs (Fastest, Most Complex)

```rust
// Rust - must match Go layout exactly
#[repr(C)]
pub struct PriceData {
    pub numerator: u64,
    pub denominator: u64,
    pub timestamp: i64,
}
```

```go
// Go - must match Rust layout
type PriceData struct {
    Numerator   uint64
    Denominator uint64
    Timestamp   int64
}
```

**Pros:** Zero overhead, direct memory access  
**Cons:** Manual alignment, no nested types, painful maintenance

#### Option 2: Cap'n Proto (Zero-Copy, Schema-Driven)

Cap'n Proto's wire format IS the in-memory format—no parsing needed.

```capnp
# schema.capnp
struct SpotPriceResponse {
  numerator @0 :Data;      # BigInt as bytes
  denominator @1 :Data;
  timestamp @2 :Int64;
}

struct GetAmountOutResponse {
  amountOut @0 :Data;
  gasCost @1 :Data;
  stateId @2 :Text;
  executionPrice @3 :SpotPriceResponse;
}
```

```rust
// Rust side - write directly to Cap'n Proto message
use capnp::message::Builder;

pub fn create_response(amount: &BigUint, gas: u64) -> Vec<u8> {
    let mut message = Builder::new_default();
    let mut resp = message.init_root::<get_amount_out_response::Builder>();
    resp.set_amount_out(&amount.to_bytes_be());
    resp.set_gas_cost(&gas.to_be_bytes());
    capnp::serialize::write_message_to_words(&message)
}
```

```go
// Go side - read directly from buffer, no parsing!
import "zombiezen.com/go/capnproto2"

func ReadResponse(data []byte) (*big.Int, error) {
    msg, _ := capnp.NewDecoder(bytes.NewReader(data)).Decode()
    resp, _ := ReadRootGetAmountOutResponse(msg)
    amountBytes, _ := resp.AmountOut()
    return new(big.Int).SetBytes(amountBytes), nil
}
```

**Latency:** ~10-50ns to "deserialize" (just pointer arithmetic)  
**Pros:** Zero-copy, schema evolution, very fast  
**Cons:** Less ecosystem support than Protobuf, binary debugging harder

#### Option 3: FlatBuffers (Zero-Copy, Google)

Similar to Cap'n Proto, developed by Google for games.

```flatbuffers
// schema.fbs
namespace Tycho;

table SpotPriceResponse {
  numerator: [ubyte];    // BigInt as bytes
  denominator: [ubyte];
  timestamp: int64;
}

table GetAmountOutResponse {
  amount_out: [ubyte];
  gas_cost: [ubyte];
  state_id: string;
  execution_price: SpotPriceResponse;
}

root_type GetAmountOutResponse;
```

```rust
// Rust side
use flatbuffers::FlatBufferBuilder;

pub fn create_response<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    amount: &[u8],
    gas: &[u8],
) -> &'a [u8] {
    let amount_vec = builder.create_vector(amount);
    let gas_vec = builder.create_vector(gas);
    
    let resp = GetAmountOutResponse::create(builder, &GetAmountOutResponseArgs {
        amount_out: Some(amount_vec),
        gas_cost: Some(gas_vec),
        state_id: None,
        execution_price: None,
    });
    builder.finish(resp, None);
    builder.finished_data()
}
```

```go
// Go side - zero-copy read
import flatbuffers "github.com/google/flatbuffers/go"

func ReadResponse(data []byte) *big.Int {
    resp := GetRootAsGetAmountOutResponse(data, 0)
    amountBytes := resp.AmountOutBytes()
    return new(big.Int).SetBytes(amountBytes)
}
```

**Latency:** ~20-100ns  
**Pros:** Zero-copy, good tooling, battle-tested (games, Android)  
**Cons:** Slightly more verbose than Cap'n Proto

#### Option 4: JSON (Current Approach)

```rust
// Rust side
let json = serde_json::to_vec(&response)?;
```

```go
// Go side
var resp GetAmountOutResponse
json.Unmarshal(data, &resp)
```

**Latency:** ~1-10μs  
**Pros:** Human readable, easy debugging, no schema files  
**Cons:** Slower, larger payloads, allocation overhead

#### Performance Comparison

| Format      | 1 field | 10 fields | 100 fields | Notes                   |
| ----------- | ------- | --------- | ---------- | ----------------------- |
| Raw struct  | ~0ns    | ~0ns      | ~0ns       | Direct memory access    |
| Cap'n Proto | ~10ns   | ~15ns     | ~50ns      | Pointer arithmetic only |
| FlatBuffers | ~20ns   | ~40ns     | ~100ns     | Slightly more overhead  |
| Protobuf    | ~100ns  | ~500ns    | ~2μs       | Full decode required    |
| JSON        | ~1μs    | ~5μs      | ~20μs      | Parse + allocate        |

#### When to Use Each

| Scenario                         | Recommended Format        |
| -------------------------------- | ------------------------- |
| >1M ops/sec needed               | Cap'n Proto / FlatBuffers |
| Complex nested data + speed      | Cap'n Proto               |
| Mobile/embedded + speed          | FlatBuffers               |
| Existing Protobuf ecosystem      | Protobuf                  |
| Debugging, prototyping           | JSON                      |
| <100K ops/sec, simplicity valued | JSON                      |

---

## 2. Shared Memory (mmap)

**Latency: ~100-500 nanoseconds**

Both processes map the same memory region, enabling zero-copy data sharing.

### How It Works

```
┌─────────────┐                    ┌─────────────┐
│   Go Code   │                    │  Rust Code  │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       └──────────┐      ┌────────────────┘
                  ▼      ▼
            ┌─────────────────┐
            │  Shared Memory  │
            │   (/dev/shm)    │
            └─────────────────┘
```

### Rust Side

```rust
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;

pub struct SharedBuffer {
    mmap: MmapMut,
}

impl SharedBuffer {
    pub fn create(path: &str, size: usize) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(size as u64)?;
        
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(Self { mmap })
    }
    
    pub fn write(&mut self, offset: usize, data: &[u8]) {
        self.mmap[offset..offset + data.len()].copy_from_slice(data);
    }
    
    pub fn read(&self, offset: usize, len: usize) -> &[u8] {
        &self.mmap[offset..offset + len]
    }
}
```

### Go Side

```go
package sharedmem

import (
    "os"
    "syscall"
)

type SharedBuffer struct {
    data []byte
    file *os.File
}

func Open(path string, size int) (*SharedBuffer, error) {
    file, err := os.OpenFile(path, os.O_RDWR|os.O_CREATE, 0644)
    if err != nil {
        return nil, err
    }
    
    if err := file.Truncate(int64(size)); err != nil {
        file.Close()
        return nil, err
    }
    
    data, err := syscall.Mmap(
        int(file.Fd()),
        0,
        size,
        syscall.PROT_READ|syscall.PROT_WRITE,
        syscall.MAP_SHARED,
    )
    if err != nil {
        file.Close()
        return nil, err
    }
    
    return &SharedBuffer{data: data, file: file}, nil
}

func (sb *SharedBuffer) Write(offset int, data []byte) {
    copy(sb.data[offset:], data)
}

func (sb *SharedBuffer) Read(offset, length int) []byte {
    return sb.data[offset : offset+length]
}

func (sb *SharedBuffer) Close() error {
    syscall.Munmap(sb.data)
    return sb.file.Close()
}
```

### Synchronization Pattern

```go
// Use atomic operations or a separate signaling channel
// Header structure at offset 0:
// - bytes 0-7: write position (atomic uint64)
// - bytes 8-15: read position (atomic uint64)
// - bytes 16+: ring buffer data
```

### Pros & Cons

| Pros                     | Cons                           |
| ------------------------ | ------------------------------ |
| Zero-copy for large data | Complex synchronization        |
| Very low latency         | Platform-specific              |
| High throughput          | Requires careful memory layout |
| Works across processes   | Harder to debug                |

### When to Use

- Streaming large datasets
- Video/audio processing pipelines
- High-frequency trading systems
- When data is too large for serialization

---

## 3. Unix Domain Sockets

**Latency: ~1-10 microseconds**

Local socket communication without TCP/IP overhead.

### How It Works

```
┌─────────────┐    Unix Socket    ┌─────────────┐
│   Go Code   │◄─────────────────►│  Rust Code  │
│  (Client)   │  /tmp/app.sock    │  (Server)   │
└─────────────┘                   └─────────────┘
```

### Rust Server

```rust
use std::os::unix::net::{UnixListener, UnixStream};
use std::io::{Read, Write};

fn handle_client(mut stream: UnixStream) {
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                // Process and respond
                let response = process(&buffer[..n]);
                stream.write_all(&response).unwrap();
            }
            Err(_) => break,
        }
    }
}

fn main() {
    let _ = std::fs::remove_file("/tmp/myapp.sock");
    let listener = UnixListener::bind("/tmp/myapp.sock").unwrap();
    
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            std::thread::spawn(|| handle_client(stream));
        }
    }
}
```

### Go Client

```go
package main

import (
    "net"
    "encoding/binary"
)

type UDSClient struct {
    conn net.Conn
}

func NewUDSClient(socketPath string) (*UDSClient, error) {
    conn, err := net.Dial("unix", socketPath)
    if err != nil {
        return nil, err
    }
    return &UDSClient{conn: conn}, nil
}

func (c *UDSClient) Call(request []byte) ([]byte, error) {
    // Write length-prefixed message
    lenBuf := make([]byte, 4)
    binary.BigEndian.PutUint32(lenBuf, uint32(len(request)))
    
    if _, err := c.conn.Write(lenBuf); err != nil {
        return nil, err
    }
    if _, err := c.conn.Write(request); err != nil {
        return nil, err
    }
    
    // Read response
    if _, err := c.conn.Read(lenBuf); err != nil {
        return nil, err
    }
    respLen := binary.BigEndian.Uint32(lenBuf)
    response := make([]byte, respLen)
    if _, err := c.conn.Read(response); err != nil {
        return nil, err
    }
    
    return response, nil
}
```

### Pros & Cons

| Pros                   | Cons                       |
| ---------------------- | -------------------------- |
| Lower latency than TCP | Local only                 |
| Standard socket API    | Still has syscall overhead |
| Process isolation      | Need serialization         |
| Easy to implement      | Connection management      |

### When to Use

- Local microservices
- Sidecar patterns
- When you need process isolation
- Plugin architectures

---

## 4. gRPC over Unix Domain Sockets

**Latency: ~10-50 microseconds**

Structured RPC with Protobuf serialization over UDS.

### How It Works

```
┌─────────────┐   gRPC/Protobuf   ┌─────────────┐
│   Go Code   │◄─────────────────►│  Rust Code  │
│  (tonic)    │    Unix Socket    │  (tonic)    │
└─────────────┘                   └─────────────┘
```

### Proto Definition

```protobuf
syntax = "proto3";
package myservice;

service Calculator {
    rpc Add(AddRequest) returns (AddResponse);
    rpc StreamNumbers(stream Number) returns (stream Number);
}

message AddRequest {
    int64 a = 1;
    int64 b = 2;
}

message AddResponse {
    int64 result = 1;
}

message Number {
    int64 value = 1;
}
```

### Rust Server (tonic)

```rust
use tonic::{transport::Server, Request, Response, Status};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;

pub mod myservice {
    tonic::include_proto!("myservice");
}

use myservice::calculator_server::{Calculator, CalculatorServer};
use myservice::{AddRequest, AddResponse};

#[derive(Default)]
pub struct MyCalculator;

#[tonic::async_trait]
impl Calculator for MyCalculator {
    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = request.into_inner();
        Ok(Response::new(AddResponse {
            result: req.a + req.b,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/tmp/calculator.sock";
    let _ = std::fs::remove_file(path);
    
    let uds = UnixListener::bind(path)?;
    let uds_stream = UnixListenerStream::new(uds);
    
    Server::builder()
        .add_service(CalculatorServer::new(MyCalculator::default()))
        .serve_with_incoming(uds_stream)
        .await?;
        
    Ok(())
}
```

### Go Client

```go
package main

import (
    "context"
    "net"
    "time"
    
    "google.golang.org/grpc"
    "google.golang.org/grpc/credentials/insecure"
    
    pb "myapp/gen/myservice"
)

func main() {
    dialer := func(ctx context.Context, addr string) (net.Conn, error) {
        return net.Dial("unix", addr)
    }
    
    conn, err := grpc.Dial(
        "/tmp/calculator.sock",
        grpc.WithTransportCredentials(insecure.NewCredentials()),
        grpc.WithContextDialer(dialer),
    )
    if err != nil {
        panic(err)
    }
    defer conn.Close()
    
    client := pb.NewCalculatorClient(conn)
    
    ctx, cancel := context.WithTimeout(context.Background(), time.Second)
    defer cancel()
    
    resp, err := client.Add(ctx, &pb.AddRequest{A: 10, B: 20})
    if err != nil {
        panic(err)
    }
    
    fmt.Printf("Result: %d\n", resp.Result)
}
```

### Pros & Cons

| Pros                             | Cons                            |
| -------------------------------- | ------------------------------- |
| Type-safe contracts              | Protobuf serialization overhead |
| Streaming support                | More complex setup              |
| Automatic code generation        | Larger binary size              |
| Works with existing gRPC tooling | HTTP/2 framing overhead         |

### When to Use

- When you already use gRPC
- Need streaming capabilities
- Want strong typing across languages
- **This is what tycho-bridge uses!**

---

## 5. gRPC over TCP

**Latency: ~50-200 microseconds**

Standard network gRPC—same as above but over TCP.

### Go Client (TCP)

```go
conn, err := grpc.Dial(
    "localhost:50051",
    grpc.WithTransportCredentials(insecure.NewCredentials()),
)
```

### When to Use

- Distributed systems
- When services run on different machines
- Need load balancing
- Service mesh integration

---

## 6. HTTP/REST

**Latency: ~100-500 microseconds**

Simple HTTP APIs with JSON/MessagePack.

### Rust Server (axum)

```rust
use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AddRequest {
    a: i64,
    b: i64,
}

#[derive(Serialize)]
struct AddResponse {
    result: i64,
}

async fn add(Json(req): Json<AddRequest>) -> Json<AddResponse> {
    Json(AddResponse { result: req.a + req.b })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/add", post(add));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Go Client

```go
package main

import (
    "bytes"
    "encoding/json"
    "net/http"
)

type AddRequest struct {
    A int64 `json:"a"`
    B int64 `json:"b"`
}

type AddResponse struct {
    Result int64 `json:"result"`
}

func Add(a, b int64) (int64, error) {
    reqBody, _ := json.Marshal(AddRequest{A: a, B: b})
    
    resp, err := http.Post("http://localhost:3000/add", "application/json", bytes.NewReader(reqBody))
    if err != nil {
        return 0, err
    }
    defer resp.Body.Close()
    
    var result AddResponse
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return 0, err
    }
    
    return result.Result, nil
}
```

### Pros & Cons

| Pros                  | Cons                          |
| --------------------- | ----------------------------- |
| Simple to implement   | Higher latency                |
| Human-readable (JSON) | JSON parsing overhead         |
| Easy debugging        | No streaming (without SSE/WS) |
| Works everywhere      | HTTP overhead                 |

### When to Use

- Public APIs
- Simple integrations
- When debugging ease is priority
- Existing HTTP infrastructure

---

## 7. Message Queues

**Latency: ~100μs - 1ms**

Asynchronous communication via message brokers.

### Options

| Queue         | Latency    | Throughput | Persistence |
| ------------- | ---------- | ---------- | ----------- |
| ZeroMQ        | ~50-100μs  | Very High  | No          |
| NATS          | ~100-200μs | Very High  | Optional    |
| Redis Streams | ~200-500μs | High       | Yes         |
| RabbitMQ      | ~500μs-1ms | Medium     | Yes         |
| Kafka         | ~1-5ms     | Very High  | Yes         |

### ZeroMQ Example (Lowest Latency)

**Rust (Server)**
```rust
use zmq;

fn main() {
    let context = zmq::Context::new();
    let responder = context.socket(zmq::REP).unwrap();
    responder.bind("ipc:///tmp/myapp.ipc").unwrap();
    
    loop {
        let msg = responder.recv_bytes(0).unwrap();
        let result = process(&msg);
        responder.send(&result, 0).unwrap();
    }
}
```

**Go (Client)**
```go
package main

import (
    zmq "github.com/pebbe/zmq4"
)

func main() {
    context, _ := zmq.NewContext()
    socket, _ := context.NewSocket(zmq.REQ)
    socket.Connect("ipc:///tmp/myapp.ipc")
    
    socket.SendBytes([]byte("hello"), 0)
    reply, _ := socket.RecvBytes(0)
    fmt.Printf("Received: %s\n", reply)
}
```

### When to Use

- Decoupled architectures
- Async processing
- Pub/sub patterns
- Work queues

---

## 8. File-based IPC

**Latency: ~1-10 milliseconds**

Simple but slow—useful for batch processing.

### Methods

- **Named Pipes (FIFOs)**: ~100μs-1ms
- **Temporary Files**: ~1-10ms
- **Memory-mapped Files**: See Shared Memory section

### Named Pipe Example

**Rust**
```rust
use std::fs::File;
use std::io::{Read, Write};
use nix::unistd::mkfifo;
use nix::sys::stat::Mode;

fn main() {
    let path = "/tmp/myfifo";
    mkfifo(path, Mode::S_IRWXU).ok();
    
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
}
```

**Go**
```go
package main

import (
    "os"
    "syscall"
)

func main() {
    path := "/tmp/myfifo"
    syscall.Mkfifo(path, 0666)
    
    file, _ := os.OpenFile(path, os.O_WRONLY, 0666)
    file.Write([]byte("hello from go"))
    file.Close()
}
```

### When to Use

- Batch processing
- Simple scripts
- When latency doesn't matter
- Debugging/logging

---

## Decision Matrix

```
                          ┌─────────────────────────────────────────────┐
                          │           Need Network Access?              │
                          └──────────────────┬────────────────────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │ No                     │                    Yes │
                    ▼                        │                        ▼
        ┌───────────────────┐               │           ┌───────────────────┐
        │ Same Process?     │               │           │ Structured RPC?   │
        └─────────┬─────────┘               │           └─────────┬─────────┘
                  │                         │                     │
        ┌─────────┼─────────┐              │           ┌─────────┼─────────┐
    Yes │         │     No  │              │       Yes │         │     No  │
        ▼         │         ▼              │           ▼         │         ▼
   ┌────────┐     │    ┌─────────┐         │      ┌────────┐     │    ┌────────┐
   │CGo+FFI │     │    │ Large   │         │      │ gRPC   │     │    │  HTTP  │
   │        │     │    │ Data?   │         │      │ (TCP)  │     │    │  REST  │
   └────────┘     │    └────┬────┘         │      └────────┘     │    └────────┘
                  │         │              │
                  │    ┌────┼────┐         │
                  │Yes │    │ No │         │
                  │    ▼    │    ▼         │
                  │┌──────┐ │ ┌──────┐     │
                  ││Shared│ │ │ UDS  │     │
                  ││Memory│ │ │/gRPC │     │
                  │└──────┘ │ │ UDS  │     │
                  │         │ └──────┘     │
                  └─────────┴──────────────┘
```

---

## Performance Benchmarks

Approximate measurements on modern hardware (Linux, x86_64):

| Method               | Latency (p50) | Latency (p99) | Throughput |
| -------------------- | ------------- | ------------- | ---------- |
| CGo + FFI            | 15ns          | 50ns          | 50M+ ops/s |
| Shared Memory        | 200ns         | 500ns         | 10M+ ops/s |
| Unix Socket (raw)    | 3μs           | 10μs          | 500K ops/s |
| gRPC (UDS)           | 25μs          | 80μs          | 100K ops/s |
| gRPC (TCP localhost) | 80μs          | 200μs         | 50K ops/s  |
| HTTP/JSON            | 150μs         | 400μs         | 20K ops/s  |
| ZeroMQ (IPC)         | 50μs          | 150μs         | 200K ops/s |

*Note: Actual numbers vary significantly based on payload size, hardware, and implementation details.*

---

## tycho-bridge Approach

This project uses **gRPC over TCP** for Go-Rust communication:

```
┌─────────────────┐      gRPC/Protobuf       ┌─────────────────────┐
│  Go Client      │◄────────────────────────►│  Rust gRPC Server   │
│  (clients/go)   │     TCP :4242            │  (tycho-bridge)     │
└─────────────────┘                          └─────────────────────┘
```

**Why gRPC?**
- Streaming support for real-time updates
- Strong typing via Protobuf
- Works across network (not just local)
- Standard tooling and ecosystem
- Bidirectional streaming for WebSocket-like functionality

See [clients/go/](../clients/go/) for the Go client implementation.

---

## Recommendations by Use Case

| Use Case                   | Recommended Method           |
| -------------------------- | ---------------------------- |
| High-frequency trading     | CGo + FFI or Shared Memory   |
| Blockchain indexer (tycho) | gRPC (streaming support)     |
| Local microservices        | gRPC over UDS                |
| Plugin system              | CGo + FFI                    |
| Web API gateway            | HTTP/REST or gRPC            |
| Async event processing     | Message Queue (NATS/ZeroMQ)  |
| Large file processing      | Shared Memory or Named Pipes |

---

## Further Reading

- [CGo Documentation](https://pkg.go.dev/cmd/cgo)
- [tonic (Rust gRPC)](https://github.com/hyperium/tonic)
- [gRPC Go](https://grpc.io/docs/languages/go/)
- [ZeroMQ Guide](https://zguide.zeromq.org/)
- [NATS Documentation](https://docs.nats.io/)

### Zero-Copy Serialization

- [Cap'n Proto](https://capnproto.org/) - Zero-copy serialization by Protobuf v2 author
- [Cap'n Proto Rust](https://github.com/capnproto/capnproto-rust)
- [Cap'n Proto Go](https://github.com/capnproto/go-capnp)
- [FlatBuffers](https://flatbuffers.dev/) - Google's zero-copy format
- [FlatBuffers Rust](https://docs.rs/flatbuffers/latest/flatbuffers/)
- [FlatBuffers Go](https://pkg.go.dev/github.com/google/flatbuffers/go)

---

*Document generated: December 2024*
