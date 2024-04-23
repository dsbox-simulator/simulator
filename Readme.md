
# DsBox
a simulator for distributed systems.

## Compiling and running

`dsbox` requires a `npm`, `cargo` and ideally `rustup` to be available for building and running. 
Additionaly a nightly version of rust must be used. In order to enable nightly using `rustup` use:
```shell
rustup override set nightly
```

*TODO* `rustup target add wasm32-wasi` 

in the project directory. After that the webapp can be compiled with 
```shell
npm install
npm run build
```
this will build and pack the webapp. Now to run `dsbox` and get a cli help message run:
```shell
cargo run
```
included in the project are three types of nodes useful for testing the program. These are in the directory `nodes` and 
can be built using:
```shell
cargo build --workspace --exclude dsbox
```
the `echo_client` and `echo_server` nodes are simple echo/reply nodes. To run the echo test use 
(assumes the nodes were built in debug mode)
```shell
cargo run -- "target/debug/echo_client" --servers "target/wasm32-wasi/debug/echo_server.wasm"
```
the `netsim_test` simply sends messages to itself and records their delay. To run the test use
```shell
cargo build --release --workspace
cargo run --release -- "target/release/netsim_test" --servers "/dev/null"
```
(for `--servers` any path may be given, since no server is launched anyway)

to run `dsbox` in interactive mode using the webapp add the `-i` flag:
```shell
cargo run -- "target/debug/echo_client" --servers "target/wasm32-wasi/debug/echo_server.wasm" -i
```
this will start a webserver on port 8080 ([http://localhost:8080]()). In debug mode, the webserver serves the files out 
of the `webapp` folder directly. This means that the webapp may be changed and the website reloaded while the `dsbox` is 
running. In release mode however, the webapp is embedded into the binary, so that it can run self-contained. 

## Nodes

A node is a communication partner in the simulated distributed system. Each node is implemented as a program, 
that uses its standard input and output to "send" and "receive" messages in JSON format to and from the "network".
Nodes can be implemented in any programming language, and can run as native executables, or as Webassembly modules,
using WASI (although with no other access to the system than standard input and output for now) 
(also: only compiled executables or wasm modules are supported for now, which excludes programming languages that require 
command line arguments to launch, like python or java). Processes can use their standard error stream to write log messages
that are shown to the user.
`dsbox` distinguishes between two kinds of nodes: 
server nodes and client nodes. 

A server node should implement the functionality that is to be tested or demonstrated. This 
can be anything, for example a consistent replicated data store or a CRDT counter or whatever else.

A client node serves as a "client" that gives the servers work to do. Multiple client nodes may generate messages 
for the servers to handle, which they then should act on accordingly. The clients may also be responsible for testing
the server's functionality. 

Each server node is an independently launched process, while all client nodes are implemented in one process 
(which can act as multiple "clients"). When `dsbox` is launched, it will launch a single process 
(using the executable passed on the commandline), which is then in charge of the "setup" for what should happen next.
It then tells the core how many clients exist in the network (and their names) and how many servers it should launch 
(again, with their names. These are launched from the executable passed in the `--servers` command line argument).
The core then registers the client and server names and launches the corresponding amount of server processes. 
After that, communication may begin. The client process may at any point send another "setup" to the core, which then
re-launches new servers, and registers new client names accordingly. After the client process finishes, `dsbox` exits.

## Messages

Messages are exchanged between nodes in JSON format. Each node writes a message that it wants to send as a single line
to its standard output. The message is then handled by the core and delivered to the destination node specified in the message.
A message is a JSON object containing the name of the source node (`src`), the name of the destination node (`dest`) 
and a message body. For example

```json
{
  "src": "c0",
  "dest": "s1",
  "body": {...}
}
```
describes a message that is sent by node `c0` and should be delivered to node `s0`. The body of a message contains 
further information the contents of the message. It _must_ include a `type` field, and can optionally include a `msg_id`
integer that can be used by the sender to identify a message (or specifically a reply to a message) and an `in_reply_to`
field, that works in tandem with the `msg_id` field. Other than that, a body can contain other arbitrary data. 
This data should ideally be specified by the `type` field in contract between the sender and receiver of the message.

```json
{
  "src": "c0",
  "dest": "s1",
  "body": {
    "type": "echo",
    "msg_id": 3,
    "echo": "hello from c0"
  }
}
```
This is a complete message of type `echo`, sent from `c0` to `s0` with id `3`. It is probably expected for `s0` to reply 
with an appropriate response (maybe of type `echo_ok`) with the `in_reply_to` field set to `3`.

### Implementing clients

A client is implemented as program that can set up a specific "test" and then act as multiple nodes. 
When the client process is first launched, the core remains idle until the process sends a `Setup` message to it.
The setup message must contain the names of all clients and servers that should be set up. It should have its `dest`
field set to `core` and its `src` field set to `client`. For example:

```json
{
  "src": "client",
  "dest": "core",
  "body": {
    "type": "setup",
    "clients": ["c0", "c1"],
    "servers": ["s0", "s1", "s2"]
  }
}
```
The server responds with a `setup_ok` message (without any additional data in the body).
After that, client and server nodes can communicate with one another, until the client process sends another `setup`
message (which then starts a new setup).

### Implementing servers

A Server is implemented as a program that will be launched multiple times to simulate multiple independent nodes in the
distributed system. Each server receives (and should wait for) a message with type `init` immediately after launch, 
containing its own name, as well as the other server names (it does not know the names of the clients in the network).
For example:
```json
{
  "src": "core",
  "dest": "s0",
  "body": {
    "type": "init",
    "name": "s1",
    "servers": ["s0", "s1", "s2"]
  }
}
```
Servers should not reply to this message. After that, they can start communications.

### Python module
In the `python` directory resides a `pynode` module which contains some convenience code to implement servers or clients.
The `Message` and `MessageBody` can be used to serialize/deserialize messages to and from json strings. Additionally
`Message.recv()` can be used to receive (blocking) a single message and `Message.recv_iter()` can be used to receive 
any number of messages as an iterable. `Message` also has some properties for e.g. the message id or type, so that
for example `message.body.type` may be shortened to `message.type`. A reply to a received message can be constructed
using the `Message.reply(self, body)` method, which returns a new `Message` with the given body, `src` and `dest` attributes swapped
and if the original message had an id, the `in_reply_to` field set. The `Message.send(self)` method prints a message as
json to the standard output. Additionally, a `log` function is provided to print logging/debug messages (to standard error)
which will show up in the cores standard output (and soon in the webapp)

A simple "echo" type server can be implemented in a few lines of python:
```python
from pynode import Message, MessageBody, log

message = Message.recv()
assert message.type == "init"
for message in Message.recv_iter():
    reply = message.reply(MessageBody('echo_ok', echo=message.body.echo))
    reply.send()
```
this server waits for the `init` message (asserting that the first message is actually an `init` message) and then
replies to every received message with an `echo_ok` message, copying the `echo` field from the received messages body.