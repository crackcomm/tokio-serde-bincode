extern crate bincode;
extern crate futures;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_serde_bincode;

use futures::{Future, Sink};

use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;

// Use length delimited frames
use tokio_io::codec::length_delimited;

use tokio_serde_bincode::WriteBincode;

#[derive(Debug, Deserialize, Serialize)]
struct Data {
    field: i32,
}

pub fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind a server socket
    let socket = TcpStream::connect(
        &"127.0.0.1:17653".parse().unwrap(),
        &handle);

    core.run(socket.and_then(|socket| {

        // Delimit frames using a length header
        let length_delimited = length_delimited::FramedWrite::new(socket);

        // Serialize frames with JSON
        let serialized: WriteBincode<length_delimited::FramedWrite<TcpStream>, Data> = WriteBincode::new(length_delimited);

        // Send the value
        serialized.send(Data { field: 42 })
    })).unwrap();
}
