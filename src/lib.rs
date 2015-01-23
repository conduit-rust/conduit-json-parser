#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, allow(unstable))]

extern crate "rustc-serialize" as rustc_serialize;

extern crate conduit;
extern crate "conduit-middleware" as middleware;
extern crate "conduit-utils" as utils;

use std::error::Error;
use rustc_serialize::Decodable;
use rustc_serialize::json::{self, Json};

use conduit::Request;
use utils::RequestDelegator;
use middleware::Middleware;

pub struct BodyReader<T>;

impl<T: Decodable + 'static> Middleware for BodyReader<T> {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let json: T = try!(decode::<T>(req.body()));

        req.mut_extensions().insert(json);
        Ok(())
    }
}

fn decode<T: Decodable>(reader: &mut Reader) -> Result<T, Box<Error+Send>> {
    let j = try!(Json::from_reader(reader).map_err(|e| Box::new(e) as Box<Error+Send>));
    let mut decoder = json::Decoder::new(j);
    Decodable::decode(&mut decoder).map_err(|e| Box::new(e) as Box<Error+Send>)
}

pub fn json_params<'a, T: Decodable + 'static>(req: &'a Request) -> Option<&'a T> {
    req.extensions().find::<T>()
}

#[cfg(test)]
mod tests {
    extern crate "conduit-test" as conduit_test;

    use {json_params, BodyReader};

    use std::collections::HashMap;
    use std::error::Error;
    use std::io::MemReader;
    use rustc_serialize::json;

    use conduit::{Request, Response, Handler, Method};
    use middleware::MiddlewareBuilder;

    #[derive(PartialEq, RustcDecodable, RustcEncodable, Show)]
    struct Person {
        name: String,
        location: String
    }

    fn handler(req: &mut Request) -> Result<Response, Box<Error+Send>> {
        let person = json_params::<Person>(req);
        let out = person.map(|p| json::encode(p).unwrap()).expect("No JSON");

        Ok(Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: Box::new(MemReader::new(out.into_bytes()))
        })
    }

    #[test]
    fn test_body_params() {
        let mut req = conduit_test::MockRequest::new(Method::Get, "/");
        req.with_body(br#"{ "name": "Alex Crichton", "location": "San Francisco" }"#);

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
