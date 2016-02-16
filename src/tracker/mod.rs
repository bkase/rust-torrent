use nom::IResult;
use std::net::Ipv4Addr;
use std::collections::HashMap;
use bencode;
use bencode::{BVal, bdict};
use peers as p;

#[derive(Debug, PartialEq)]
pub struct Announcement {
    complete: Option<i32>,
    downloaded: Option<i32>,
    incomplete: Option<i32>,
    interval: i32,
    peers: Vec<p::Peer>,
    // TODO: extensions
}

static ROOT_KEY: &'static str = "_root_";
static COMPLETE_KEY: &'static str = "complete";
static DOWNLOADED_KEY: &'static str = "downloaded";
static INCOMPLETE_KEY: &'static str = "incomplete";
static INTERVAL_KEY: &'static str = "interval";
static PEERS_KEY: &'static str = "peers";
static FAILURE_REASON_KEY: &'static str = "failure reason";

#[derive(Debug)]
pub enum Error {
    MissingKey(&'static str),
    BadPeerFormat,
    BencodeValError{for_key: &'static str, err: bencode::ReadError},
    ExtraBytes,
    MissingBytes,
    BencodeParseError,
    TrackerReason(String)
}

pub fn parse(contents: &[u8]) -> Result<Announcement, Error> {
    match bdict(contents) {
        IResult::Done(rest, bv) =>
            if rest == &b""[..] {
               announce_from_bval(bv)
            } else {
               Err(Error::ExtraBytes)
            },
        IResult::Incomplete(_) => Err(Error::MissingBytes),
        IResult::Error(_) => Err(Error::BencodeParseError),
    }
}

fn get_opt_i32<'a>(m: &HashMap<&'a str, BVal<'a>>, key: &'static str) -> Result<i32, Error> {
    return m.get(key)
            .ok_or(Error::MissingKey(key))
            .and_then(|c| c.as_bint()
                      .map(|i| i as i32)
                      .or_else(|e| Err(Error::BencodeValError{for_key: key, err: e})))
}

fn announce_from_bval<'a>(bv: BVal<'a>) -> Result<Announcement, Error> {
    bv.as_bdict()
        .or_else(|e| Err(Error::BencodeValError{for_key: ROOT_KEY, err: e}))
        .and_then(|m| {
        let complete_opt = get_opt_i32(&m, COMPLETE_KEY);
        let incomplete_opt = get_opt_i32(&m, INCOMPLETE_KEY);
        let downloaded_opt = get_opt_i32(&m, DOWNLOADED_KEY);
        let interval_opt = get_opt_i32(&m, INTERVAL_KEY);

        let peers_opt = m.get(PEERS_KEY)
            .ok_or(Error::MissingKey(PEERS_KEY))
            .and_then(|p| p.as_bstring_bytes()
                      .or_else(|e| Err(Error::BencodeValError{for_key: PEERS_KEY, err: e})))
            .and_then(|bs| 
                      if bs.len() % 6 != 0 {
                        Err(Error::BadPeerFormat)
                      } else {
                          let peers: Vec<p::Peer> = bs.chunks(6)
                              .map(|chunk| {
                                  (Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]), 
                                     ((chunk[4] as u16) << 8) + chunk[5] as u16)
                              })
                              .collect();
                          Ok(peers)
                      });

        // TODO: For paralellism, a zip here would be better
        interval_opt.and_then(|interval| {
            peers_opt.map(|peers| Announcement{ 
                complete: complete_opt.ok(),
                downloaded: downloaded_opt.ok(),
                incomplete: incomplete_opt.ok(),
                interval: interval,
                peers: peers,
            })
        })
        // TODO: loko for FAILURE_REASON
        /*res.or_else(|e| m.get(FAILURE_REASON_KEY)
                        .ok_or(e)
                        .and_then(|r| r.as_bstring_str()
                                  .or_else(|e_| Err(e))
                                  .and_then(|reason| Err(Error::TrackerReason(String::from_utf8(reason))))))*/
    })
}


// for now
//
//

use std::cmp::max;
use std::str::from_utf8;
use std::net::ToSocketAddrs;
use std::io::{stdout, Write};

use rotor;
use httparse;
use rotor::mio::tcp::{TcpStream};
use rotor::{Scope};
use time::{SteadyTime, Duration};
use tracker;
use metainfo;
use sha1bytes;
use rand;
use rand::Rng;

use rotor_http::client::{connect_tcp, Request, Head, Client, RecvMode};
use rotor_http::client::{Context as HttpCtx};
use rotor_http::version::HttpVersion;
use rotor_http::method::Method;
use rotor_http::Deadline;

struct Context;
impl HttpCtx for Context {}

struct Req(String);

impl Client for Req {
    type Context = Context;
    fn prepare_request(self, req: &mut Request) -> Option<Self> {
        req.start(Method::Get, &self.0, HttpVersion::Http11);
        req.done_headers().unwrap();
        req.done();
        Some(self)
    }
    fn headers_received(self, head: Head, _request: &mut Request,
                        _scope: &mut Scope<Self::Context>)
        -> Option<(Self, RecvMode, Deadline)> {
            println!("----- Headers -----");
            println!("Status: {} {}", head.code, head.reason);
            for header in head.headers {
                println!("{}: {}", header.name,
                         String::from_utf8_lossy(header.value));
            }
            Some((self,  RecvMode::Buffered(16386), Deadline::now() +
                  Duration::seconds(1000)))
    }
    fn response_received(self, data: &[u8], _request: &mut Request,
                         scope: &mut Scope<Self::Context>) {
        println!("----- Response -----");
        match parse(data) {
            Err(e) => panic!("Got err {:?}", e),
            Ok(announcement) => panic!("Look {:?}", announcement)
        };
        scope.shutdown_loop();
    }
    fn response_chunk(self, _chunk: &[u8], _request: &mut Request,
                      _scope: &mut Scope<Self::Context>) -> Option<Self> {
        unreachable!();
    }
    fn response_end(self, _request: &mut Request,
                    _scope: &mut Scope<Self::Context>) {
        unreachable!();
    }
    fn timeout(self, _request: &mut Request, _scope: &mut Scope<Self::Context>)
        -> Option<(Self, Deadline)> {
            unreachable!();
    }
    fn wakeup(self, _request: &mut Request, _scope: &mut Scope<Self::Context>)
        -> Option<Self> {
            unimplemented!();
    }
}

pub fn start_every_interval(mi: metainfo::Metainfo) {
    let mut rand = rand::thread_rng();
    let rand_id: Vec<u8> = rand.gen_iter::<u8>().take(20).collect();
    let peer_id = sha1bytes::SHA1Hash::from_prehashed(&rand_id[..]).to_url_escaped_string();

    let path = mi.announce_path();
    let (domain, port) = mi.tracker_domain_port();
    let total_size = mi.total_size();
    let info_hash = mi.info.info_hash.to_url_escaped_string();
    let fullpath = format!("/{}?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&numwant=7&event=started", path, info_hash, peer_id, port, total_size);

    let event_loop = rotor::Loop::new(&rotor::Config::new()).unwrap();
    let addr = (domain, port).to_socket_addrs()
        .map(|mut addrs| addrs.next().unwrap())
        .unwrap();
    let mut loop_inst = event_loop.instantiate(Context);
    loop_inst.add_machine_with(|scope| {
        connect_tcp(scope, &addr, Req(fullpath.to_string()))
    }).unwrap();
    loop_inst.run().unwrap();
}
