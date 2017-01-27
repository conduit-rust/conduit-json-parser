#![cfg_attr(test, deny(warnings))]

extern crate rustc_serialize;

extern crate conduit;
extern crate conduit_middleware as middleware;
extern crate conduit_utils as utils;

use std::any::Any;
use std::error::Error;
use std::io::prelude::*;
use std::marker;
use rustc_serialize::Decodable;
use rustc_serialize::json::{self, Json};

use conduit::Request;
use middleware::Middleware;

pub struct BodyReader<T> {
    _marker: marker::PhantomData<fn() -> T>,
}

impl<T: Decodable + Any> BodyReader<T> {
    pub fn new() -> BodyReader<T> {
        BodyReader { _marker: marker::PhantomData }
    }
}

impl<T: Decodable + Any> Middleware for BodyReader<T> {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let json: T = try!(decode::<T>(req.body()));

        req.mut_extensions().insert(json);
        Ok(())
    }
}

#[allow(trivial_casts)]
fn decode<T: Decodable>(reader: &mut Read) -> Result<T, Box<Error+Send>> {
    let j = try!(Json::from_reader(reader).map_err(|e| Box::new(e) as Box<Error+Send>));
    let mut decoder = json::Decoder::new(j);
    Decodable::decode(&mut decoder).map_err(|e| Box::new(e) as Box<Error+Send>)
}

pub fn json_params<'a, T: Decodable + Any>(req: &'a Request) -> Option<&'a T> {
    req.extensions().find::<T>()
}

#[cfg(test)]
mod tests {
    extern crate conduit_test;

    use {json_params, BodyReader};

    use std::collections::HashMap;
    use std::io::{self, Cursor};
    use rustc_serialize::json;

    use conduit::{Request, Response, Handler, Method};
    use middleware::MiddlewareBuilder;

    #[derive(PartialEq, RustcDecodable, RustcEncodable, Debug)]
    struct Person {
        name: String,
        location: String
    }

    fn handler(req: &mut Request) -> io::Result<Response> {
        let person = json_params::<Person>(req);
        let out = person.map(|p| json::encode(p).unwrap()).expect("No JSON");

        Ok(Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: Box::new(Cursor::new(out.into_bytes()))
        })
    }

    #[test]
    fn test_body_params() {
        let mut req = conduit_test::MockRequest::new(Method::Get, "/");
        req.with_body(br#"{ "name": "Alex Crichton", "location": "San Francisco" }"#);

        let mut middleware = MiddlewareBuilder::new(handler);
        middleware.add(BodyReader::<Person>::new());

        let mut res = middleware.call(&mut req).ok().expect("No response");
        let mut body = Vec::new();
        res.body.write_body(&mut body).unwrap();
        let person = super::decode::<Person>(&mut &body[..]).ok().expect("No JSON response");
        assert_eq!(person, Person {
            name: "Alex Crichton".to_string(),
            location: "San Francisco".to_string()
        });
    }
}
