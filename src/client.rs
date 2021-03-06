// Rust JSON-RPC Library
// Written in 2015 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! # Client support
//!
//! Support for connecting to JSONRPC servers over HTTP, sending requests,
//! and parsing responses
//!

use std::sync::{Arc, Mutex};

use hyper::client::Client as HyperClient;
use hyper::header::{Headers, Authorization, Basic};
use hyper::status::StatusCode;
use json;
use json::value::Value as JsonValue;

use super::{Request, Response};
use error::Error;

/// A handle to a remote JSONRPC server
pub struct Client {
    url: String,
    user: Option<String>,
    pass: Option<String>,
    client: HyperClient,
    nonce: Arc<Mutex<u64>>
}

impl Client {
    /// Creates a new client
    pub fn new(url: String, user: Option<String>, pass: Option<String>) -> Client {
        // Check that if we have a password, we have a username; other way around is ok
        debug_assert!(pass.is_none() || user.is_some());

        Client {
            url: url,
            user: user,
            pass: pass,
            client: HyperClient::new(),
            nonce: Arc::new(Mutex::new(0))
        }
    }

    /// Sends a request to a client
    pub fn send_request(&self, request: &Request) -> Result<Response, Error> {
        // Build request
        let request = json::to_string(&request).unwrap();

        // Setup connection
        let mut headers = Headers::new();
        if let Some(ref user) = self.user {
            headers.set(Authorization(Basic {
                username: user.clone(),
                password: self.pass.clone()
            }));
        }

        // Send request
        let request = self.client.post(&self.url).headers(headers).body(&request);
        let stream = try!(request.send().map_err(Error::Hyper));
        if stream.status == StatusCode::Ok {
            // TODO check nonces match
            json::de::from_reader(stream).map_err(Error::Json)
        } else {
            Err(Error::BadStatus(stream.status))
        }
    }

    /// Builds a request
    pub fn build_request(&self, name: String, params: Vec<JsonValue>) -> Request {
        let mut nonce = self.nonce.lock().unwrap();
        *nonce += 1;
        Request {
            method: name,
            params: params,
            id: JsonValue::U64(*nonce)
        }
    }

    /// Accessor for the last-used nonce
    pub fn last_nonce(&self) -> u64 {
        *self.nonce.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        let client = Client::new("localhost".to_owned(), None, None);
        assert_eq!(client.last_nonce(), 0);
        let req1 = client.build_request("test".to_owned(), vec![]);
        assert_eq!(client.last_nonce(), 1);
        let req2 = client.build_request("test".to_owned(), vec![]);
        assert_eq!(client.last_nonce(), 2);
        assert!(req1 != req2);
    }
}

