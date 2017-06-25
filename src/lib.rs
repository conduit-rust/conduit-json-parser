#![cfg_attr(test, deny(warnings))]

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate conduit;
extern crate conduit_middleware as middleware;
extern crate conduit_utils as utils;

use std::any::Any;
use std::error::Error;
use std::io::prelude::*;
use std::marker;

use serde::Deserialize;

use conduit::Request;
use middleware::Middleware;

pub struct BodyReader<T> {
    _marker: marker::PhantomData<fn() -> T>,
}

impl<'de, T: Deserialize<'de> + Any> BodyReader<T> {
    pub fn new() -> BodyReader<T> {
        BodyReader { _marker: marker::PhantomData }
    }
}

impl<'de, T: Deserialize<'de> + Any> Middleware for BodyReader<T> {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        let json: T = deserialize::<T>(req.body())?;
        req.mut_extensions().insert(json);
        Ok(())
    }
}

#[allow(trivial_casts)]
fn deserialize<'de, T: Deserialize<'de>>(reader: &mut Read) -> Result<T, Box<Error+Send>> {
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    Deserialize::deserialize(&mut deserializer).map_err(|e| Box::new(e) as Box<Error+Send>)
}

pub fn json_params<'de, 'a: 'de, T: Deserialize<'de> + Any>(req: &'a Request) -> Option<&'a T> {
    req.extensions().find::<T>()
}

#[cfg(test)]
mod tests {
    extern crate serde_json;
    extern crate conduit_test;

    use {json_params, BodyReader};

    use std::collections::HashMap;
    use std::io::{self, Cursor};

    use conduit::{Request, Response, Handler, Method};
    use middleware::MiddlewareBuilder;

    #[derive(PartialEq, Deserialize, Serialize, Debug)]
    struct Person {
        name: String,
        location: String
    }

    fn handler(req: &mut Request) -> io::Result<Response> {
        let person = json_params::<Person>(req);
        let out = person.map(|p| serde_json::to_string(p).unwrap()).expect("No JSON");

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
        let person = super::deserialize::<Person>(&mut &body[..]).ok().expect("No JSON response");
        assert_eq!(person, Person {
            name: "Alex Crichton".to_string(),
            location: "San Francisco".to_string()
        });
    }
}
