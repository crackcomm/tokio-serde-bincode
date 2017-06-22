extern crate bincode;
extern crate bytes;
extern crate futures;
extern crate serde;
extern crate tokio_serde;

use std::io;
use std::marker::PhantomData;

use bincode::{Infinite, deserialize_from, serialize};
use bytes::{Buf, Bytes, BytesMut, IntoBuf};
use futures::{Poll, Sink, StartSend, Stream};
use serde::{Deserialize, Serialize};
use tokio_serde::{Deserializer, FramedRead, FramedWrite, Serializer};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Serde(bincode::Error),
}

impl From<io::Error> for Error {
    fn from(src: io::Error) -> Self {
        Error::Io(src)
    }
}

struct Bincode<T> {
    ghost: PhantomData<T>,
}

impl<T> Deserializer<T> for Bincode<T>
where for<'de> T: Deserialize<'de>,
{
    type Error = Error;

    fn deserialize(&mut self, src: &Bytes) -> Result<T, Error> {
        deserialize_from(&mut src.into_buf().reader(), Infinite)
            .map_err(Error::Serde)
    }
}

impl<T> Serializer<T> for Bincode<T>
where T: Serialize
{
    type Error = io::Error;

    fn serialize(&mut self, item: &T) -> Result<BytesMut, io::Error> {
        serialize(item, Infinite)
            .map(Into::into)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }
}

pub struct WriteBincode<T: Sink, U> {
    inner: FramedWrite<T, U, Bincode<U>>,
}

impl<T, U> WriteBincode<T, U>
where T: Sink<SinkItem=BytesMut, SinkError=io::Error>,
      U: Serialize,
{
    pub fn new(inner: T) -> Self {
        let bincode = Bincode {
            ghost: PhantomData,
        };
        WriteBincode {
            inner: FramedWrite::new(inner, bincode),
        }
    }
}

impl<T: Sink, U> WriteBincode<T, U> {
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
}

impl<T, U> Sink for WriteBincode<T, U>
where T: Sink<SinkItem=BytesMut, SinkError=io::Error>,
      U: Serialize,
{
    type SinkItem = U;
    type SinkError = io::Error;

    fn start_send(&mut self, item: U) -> StartSend<U, io::Error> {
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        self.inner.poll_complete()
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        self.inner.close()
    }
}

impl<T, U> Stream for WriteBincode<T, U>
where T: Stream + Sink,
{
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<T::Item>, T::Error> {
        self.get_mut().poll()
    }
}

pub struct ReadBincode<T, U> {
    inner: FramedRead<T, U, Bincode<U>>,
}

impl<T, U> ReadBincode<T, U>
where T: Stream<Error=io::Error>,
      for<'de> U: Deserialize<'de>,
      Bytes: From<T::Item>,
{
    pub fn new(inner: T) -> Self {
        let bincode = Bincode {
            ghost: PhantomData,
        };
        ReadBincode {
            inner: FramedRead::new(inner, bincode),
        }
    }
}

impl<T, U> ReadBincode<T, U> {
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
}

impl<T, U> Stream for ReadBincode<T, U>
where T: Stream<Error=io::Error>,
      for<'de> U: Deserialize<'de>,
      Bytes: From<T::Item>,
{
    type Item = U;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<U>, Error> {
        self.inner.poll()
    }
}

impl<T: Sink, U> Sink for ReadBincode<T, U> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(&mut self, item: T::SinkItem) -> StartSend<T::SinkItem, T::SinkError> {
        self.get_mut().start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), T::SinkError> {
        self.get_mut().poll_complete()
    }

    fn close(&mut self) -> Poll<(), T::SinkError> {
        self.get_mut().close()
    }
}
