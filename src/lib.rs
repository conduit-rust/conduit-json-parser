#![feature(globs)]
#![cfg_attr(test, deny(warnings))]

extern crate "rustc-serialize" as rustc_serialize;

extern crate conduit;
extern crate "conduit-middleware" as middleware;
extern crate "conduit-utils" as utils;

use std::fmt;
use std::fmt::{Show, Formatter};
use rustc_serialize::Decodable;
use rustc_serialize::json::{mod, Json};

use conduit::Request;
use utils::RequestDelegator;
use middleware::Middleware;

pub struct BodyReader<T>;

pub trait JsonDecodable : Decodable<json::Decoder, json::DecoderError> {}
impl<T: Decodable<json::Decoder, json::DecoderError>> JsonDecodable for T {}

impl<T: JsonDecodable + 'static> Middleware for BodyReader<T> {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        let json: T = try!(decode::<T>(req.body()).map_err(|err| {
            let s: Box<String> = box format!("Couldn't parse JSON: {}", show(&*err));
            s as Box<Show + 'static>
        }));

        req.mut_extensions().insert(json);
        Ok(())
    }
}

// Hack around the lack of impl Show for Box<Show>
struct Shower<'a> {
    inner: &'a (Show + 'a),
}

impl<'a> Show for Shower<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

fn show<'a>(s: &'a Show) -> Shower<'a> {
    Shower { inner: s }
}

fn decode<T: JsonDecodable + 'static>(reader: &mut Reader) -> Result<T, Box<Show>> {
    let j = try!(Json::from_reader(reader).map_err(|e| box e as Box<Show>));
    let mut decoder = json::Decoder::new(j);
    Decodable::decode(&mut decoder).map_err(|e| box e as Box<Show>)
}

pub fn json_params<'a, T: JsonDecodable + 'static>(req: &'a Request) -> Option<&'a T> {
    req.extensions().find::<T>()
}

#[cfg(test)]
mod tests {
    extern crate "conduit-test" as conduit_test;

    use {json_params, BodyReader};

    use std::collections::HashMap;
    use std::io::MemReader;
    use rustc_serialize::json;

    use conduit::{Request, Response, Handler, Method};
    use middleware::MiddlewareBuilder;

    #[deriving(PartialEq, RustcDecodable, RustcEncodable, Show)]
    struct Person {
        name: String,
        location: String
    }

    fn handler(req: &mut Request) -> Result<Response, ()> {
        let person = json_params::<Person>(req);
        let out = person.map(|p| json::encode(p)).expect("No JSON");

        Ok(Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box MemReader::new(out.into_bytes()) as Box<Reader + Send>
        })
    }

    #[test]
    fn test_body_params() {
        let mut req = conduit_test::MockRequest::new(Method::Get, "/");
        req.with_body(r#"{ "name": "Alex Crichton", "location": "San Francisco" }"#);

        let mut middleware = MiddlewareBuilder::new(handler);
        middleware.add(BodyReader::<Person>);

        let mut res = middleware.call(&mut req).ok().expect("No response");
        let person = super::decode::<Person>(&mut *res.body).ok().expect("No JSON response");
        assert_eq!(person, Person {
            name: "Alex Crichton".to_string(),
            location: "San Francisco".to_string()
        });
    }
}
