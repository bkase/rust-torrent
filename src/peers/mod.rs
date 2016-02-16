use metainfo;
use bit_set::BitSet;

use rotor::mio::tcp::{TcpListener, TcpStream};
use time::{SteadyTime, Duration, Deadline};
use rotor::{Scope};
use rotor_stream::{Accept, Stream, Protocol, Request, Transport};
use rotor_stream::{Expectation as E};

pub type Peer = (Ipv4Addr, u16);

struct Context {
    mi: &metainfo::Metainfo,
    have: BitSet;
    myState: StatePerSide,
    peer_id: SHA1Hash,
    peerState: StatePerSide,
}

struct StatePerSide {
    chocked: bool,
    interested: bool
}
impl StatePerSide {
    pub fn new() -> self {
        return StatePerSide{ chocked: true, interested: false }
    }
}

enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
    Port
}

enum Reply {
    Yes,
    No
}

enum BitTorrent {
    WantClose,
    HandshakeStart,
    Handshake(Reply, u8),
    MessageHeader,
    Message(u32, Message)
}

enum Seed {
    Listen,
    Connect
}

// the number of bytes in a handshake message not counting pstr-len
const HANDSHAKE_REST_SIZE = 49

impl Protocol from BitTorrent {
    type Context = Context;
    type Socket = TcpStream;
    type Seed = Seed;

    use BitTorrent as BT;

    fn in_seconds(secs u32) -> Deadline {
        return SteadyTime.now() + Duration.seconds(secs);
    }

    fn create(seed: Self::Seed, _sock: &mut TcpStream, _scope: &mut Scope<Context>) 
        -> Request<Self> {
        match seed {
            Listen => 
                Some((BT::HandshakeStart, E::Bytes(1), in_seconds(10)),
            Connect => 
                Some((BT::SendHandshake, E::Flush(0), in_seconds(10)))
        }
    }

    fn bytes_read(self, transport: &mut Transport<TcpStream>, 
                  _end: usize, scope: &mut Scope<Context>) -> Request<Self> {
        match self {
            HandshakeStart(reply) => {
                let pstrlen = transport.input()[0];
                let len = pstrlen + HANDSHAKE_REST_SIZE;
                transport.consume(1);
                Some((BT::Handshake(reply, len), E::Bytes(len), in_seconds(10)))
            }
            Handshake(reply, len) => {
                let handshake = {
                    let buf = &transport.input()[..len];
                    parse_handshake(buf)
                }
                scope.peer_id = handshake.peer_id;
                if (handshake.pstr == "BitTorrent protocol" &&
                    handshake.info_hash == scope.info_hash) {
                    match reply {
                        Yes => 
                            // TODO
                            Some((BT::Handshake(Reply::No, E::Flush(ko)
                    }
                } else {
                    println!("Dropping connection due to bad handshake")
                    None
                }
            },
            MessageHeader,
            Message(u32, Message)
        }
    }

    fn bytes_flushed(self, _transport: &mut Transport<TcpStream>, 
                     _scope: &mut Scope<Context>) -> Request<Self> {
        match self {

        }
    }

    fn timeout(self, _transport: &mut Transport<TcpStream>,
                       _scope: &mut Scope<Context>) -> Request<Self> {

    }

    fn wakeup(self, _transport: &mut Transport<TcpStream>,
              _scope: &mut Scope<Context>) -> Request<Self> {

    }
}

