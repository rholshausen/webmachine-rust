# webmachine-rust

Port of Webmachine-Ruby (https://github.com/webmachine/webmachine-ruby) to Rust.

[![Build Status](https://travis-ci.org/uglyog/webmachine-rust.svg?branch=master)](https://travis-ci.org/uglyog/webmachine-rust)

webmachine-rust is a port of the Ruby version of webmachine. It implements a finite state machine for the HTTP protocol
that provides semantic HTTP handling (based on the [diagram from the webmachine project](https://webmachine.github.io/images/http-headers-status-v3.png)).
It is basically a HTTP toolkit for building HTTP-friendly applications using the [Hyper](https://crates.io/crates/hyper) rust crate.

Webmachine-rust works with Hyper and sits between the Hyper Handler and your application code. It provides a resource struct
with callbacks to handle the decisions required as the state machine is executed against the request with the following sequence.

REQUEST -> Hyper Handler -> WebmachineDispatcher -> WebmachineResource -> Your application code -> WebmachineResponse -> Hyper -> RESPONSE

## Features

- Handles the hard parts of content negotiation, conditional requests, and response codes for you.
- Provides a resource struct with points of extension to let you describe what is relevant about your particular resource.

## Missing Features

Currently, the following features from webmachine-ruby have not been implemented:

- Visual debugger
- Streaming response bodies

## Implementation Deficiencies:

This implementation has the following deficiencies:

- Automatically decoding request bodies and encoding response bodies.
- No easy mechanism to generate bodies with different content types (e.g. JSON vs. XML).
- No easy mechanism for handling sub-paths in a resource.
- Dynamically determining the methods allowed on the resource.

## Getting started with Hyper

Follow the getting started documentation from the Hyper crate to setup a Hyper service for your server.
You need to define a WebmachineDispatcher that maps resource paths to your webmachine resources (WebmachineResource).
Each WebmachineResource defines all the callbacks (via Closures) and values required to implement a resource.

Note: This example uses the maplit crate to provide the `btreemap` macro and the log crate for the logging macros.

```rust
use std::convert::Infallible;
use std::future::ready;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::FutureExt;
use hyper::{body, Request};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use maplit::btreemap;
use serde_json::{Value, json};
use tracing::error;
use tokio::net::TcpListener;
use webmachine_rust::*;
use webmachine_rust::context::*;
use webmachine_rust::headers::*;

async fn start_server() -> anyhow::Result<()> {
  // setup the dispatcher, which maps paths to resources. We wrap it in an Arc so we can
  // use it in the loop below.
  let dispatcher = Arc::new(WebmachineDispatcher {
    routes: btreemap!{
          "/myresource" => WebmachineDispatcher::box_resource(WebmachineResource {
            // Methods allowed on this resource
            allowed_methods: owned_vec(&["OPTIONS", "GET", "HEAD", "POST"]),
            
            // if the resource exists callback
            resource_exists: callback(|_, _| true),
            
            // callback to render the response for the resource, it has to be async
            render_response: async_callback(|_, _| {
                let json_response = json!({
                   "data": [1, 2, 3, 4]
                });
                ready(Ok(Some(Bytes::from(json_response.to_string())))).boxed()
            }),
            
            // callback to process the post for the resource
            process_post: async_callback(|_, _|  /* Handle the post here */ ready(Ok(true)).boxed() ),
            
            // default everything else
            .. WebmachineResource::default()
          })
      }
  });

  // Create a Hyper server that delegates to the dispatcher. See https://hyper.rs/guides/1/server/hello-world/
  let addr: SocketAddr = "0.0.0.0:8080".parse()?;
  let listener = TcpListener::bind(addr).await?;
  loop {
    let dispatcher = dispatcher.clone();
    let (stream, _) = listener.accept().await?;
    let io = TokioIo::new(stream);
    tokio::task::spawn(async move {
      if let Err(err) = http1::Builder::new()
        .serve_connection(io, service_fn(|req: Request<body::Incoming>| dispatcher.dispatch(req)))
        .await
      {
        error!("Error serving connection: {:?}", err);
      }
    });
  }
  Ok(())
}
 ```

## Using the Resource trait

You can create your own struct that implements the `Resource` trait instead of using `WebmachineResource`. This avoids
needing to wrap the callbacks.

for instance, the example above can be changed to:

```rust
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use maplit::btreemap;
use serde_json::json;
use webmachine_rust::*;
use webmachine_rust::context::WebmachineContext;

#[derive(Debug)]
struct MyResource;

#[async_trait]
impl Resource for MyResource {
    fn allowed_methods(&self) -> Vec<&str> {
        vec!["OPTIONS", "GET", "HEAD", "POST"]
    }

    async fn resource_exists(&self, _context: &mut WebmachineContext) -> bool {
        true
    }

    async fn render_response(&self, _context: &mut WebmachineContext) -> anyhow::Result<Option<Bytes>> {
        let json_response = json!({
           "data": [1, 2, 3, 4]
        });
        Ok(Some(Bytes::from(json_response.to_string())))
    }

    async fn process_post(&self, _context: &mut WebmachineContext) -> Result<bool, u16> {
        Ok(true)
    }
}

async fn start_server() -> anyhow::Result<()> {
    let dispatcher = Arc::new(WebmachineDispatcher {
        routes: btreemap! {
          "/myresource" => WebmachineDispatcher::box_resource(MyResource)
      }
    });
    
    // ....
    
    Ok(())
}
```

## Example implementations

For an example of a project using this crate, have a look at the [Pact Mock Server](https://github.com/pact-foundation/pact-core-mock-server/tree/main/pact_mock_server_cli) from the Pact reference implementation.
