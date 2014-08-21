#![feature(globs)]

extern crate serialize;

extern crate conduit;
extern crate middleware = "conduit-middleware";
extern crate utils = "conduit-utils";

use std::fmt;
use std::fmt::{Show, Formatter};
use serialize::{Decodable, json};

use conduit::Request;
use utils::RequestDelegator;
use middleware::Middleware;

pub struct BodyReader<T>;

trait JsonDecodable : Decodable<json::Decoder, json::DecoderError> {}
impl<T: Decodable<json::Decoder, json::DecoderError>> JsonDecodable for T {}

impl<T: JsonDecodable + 'static> Middleware for BodyReader<T> {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show>> {
        let json: T = try!(decode::<T>(req.body()).map_err(|err| {
            box format!("Couldn't parse JSON: {}", show(err)) as Box<Show>
        }));

        req.mut_extensions().insert(json);
        Ok(())
    }
}

// Hack around the lack of impl Show for Box<Show>
struct Shower<'a> {
    inner: &'a Show
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
    let j = try!(json::from_reader(reader).map_err(|e| box e as Box<Show>));
    let mut decoder = json::Decoder::new(j);
    Decodable::decode(&mut decoder).map_err(|e| box e as Box<Show>)
}

pub fn json_params<'a, T: JsonDecodable + 'static>(req: &'a Request) -> Option<&'a T> {
    req.extensions().find::<T>()
}

#[cfg(test)]
mod tests {
    extern crate conduit_test = "conduit-test";

    use {json_params, BodyReader};

    use std::collections::HashMap;
    use std::io::MemReader;
    use serialize::json;

    use conduit;
    use conduit::{Request, Response, Handler};
    use middleware::MiddlewareBuilder;

    #[deriving(PartialEq, Decodable, Encodable, Show)]
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
        let mut req = conduit_test::MockRequest::new(conduit::Get, "/");
        req.with_body(r#"{ "name": "Alex Crichton", "location": "San Francisco" }"#);

        let mut middleware = MiddlewareBuilder::new(handler);
        middleware.add(BodyReader::<Person>);

        let mut res = middleware.call(&mut req).ok().expect("No response");
        let person = super::decode::<Person>(res.body).ok().expect("No JSON response");
        assert_eq!(person, Person {
            name: "Alex Crichton".to_string(),
            location: "San Francisco".to_string()
        });
    }
}
