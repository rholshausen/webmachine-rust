/*!
# webmachine-rust

Port of Webmachine-Ruby (https://github.com/webmachine/webmachine-ruby) to Rust.

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

 ```no_run
 use webmachine_rust::*;
 use webmachine_rust::context::*;
 use webmachine_rust::headers::*;
 use bytes::Bytes;
 use serde_json::{Value, json};
 use std::io::Read;
 use std::net::SocketAddr;
 use std::convert::Infallible;
 use std::sync::Arc;
 use maplit::btreemap;
 use tracing::error;
 use hyper_util::rt::TokioIo;
 use tokio::net::TcpListener;
 use hyper::server::conn::http1;
 use hyper::service::service_fn;
 use hyper::{body, Request};

 # fn main() {}

 async fn start_server() -> anyhow::Result<()> {
   // setup the dispatcher, which maps paths to resources. We wrap it in an Arc so we can
   // use it in the loop below.
   let dispatcher = Arc::new(WebmachineDispatcher {
       routes: btreemap!{
          "/myresource" => WebmachineResource {
            // Methods allowed on this resource
            allowed_methods: owned_vec(&["OPTIONS", "GET", "HEAD", "POST"]),
            // if the resource exists callback
            resource_exists: callback(|_, _| true),
            // callback to render the response for the resource
            render_response: callback(|_, _| {
                let json_response = json!({
                   "data": [1, 2, 3, 4]
                });
                Some(Bytes::from(json_response.to_string()))
            }),
            // callback to process the post for the resource
            process_post: callback(|_, _|  /* Handle the post here */ Ok(true) ),
            // default everything else
            .. WebmachineResource::default()
          }
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

## Example implementations

For an example of a project using this crate, have a look at the [Pact Mock Server](https://github.com/pact-foundation/pact-core-mock-server/tree/main/pact_mock_server_cli) from the Pact reference implementation.
*/

#![warn(missing_docs)]

use std::collections::{BTreeMap, HashMap};

use bytes::Bytes;
use chrono::{DateTime, FixedOffset, Utc};
use http::{HeaderMap, Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use itertools::Itertools;
use lazy_static::lazy_static;
use maplit::hashmap;
use tracing::{debug, error, trace};

use context::{WebmachineContext, WebmachineRequest, WebmachineResponse};
use headers::HeaderValue;

#[macro_use] pub mod headers;
pub mod context;
pub mod content_negotiation;

/// Type of a Webmachine resource callback
pub type WebmachineCallback<T> = Box<dyn Fn(&mut WebmachineContext, &WebmachineResource) -> T + Send + Sync>;

/// Wrap a callback in a structure that is safe to call between threads
pub fn callback<T, RT>(cb: T) -> WebmachineCallback<RT>
  where T: Fn(&mut WebmachineContext, &WebmachineResource) -> RT + Send + Sync + 'static {
  Box::new(cb)
}

/// Convenience function to create a vector of string structs from a slice of strings
pub fn owned_vec(strings: &[&str]) -> Vec<String> {
  strings.iter().map(|s| s.to_string()).collect()
}

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {
  /// This is called just before the final response is constructed and sent. It allows the resource
  /// an opportunity to modify the response after the webmachine has executed.
  pub finalise_response: Option<WebmachineCallback<()>>,
  /// This is invoked to render the response for the resource
  pub render_response: WebmachineCallback<Option<Bytes>>,
  /// Is the resource available? Returning false will result in a '503 Service Not Available'
  /// response. Defaults to true. If the resource is only temporarily not available,
  /// add a 'Retry-After' response header.
  pub available: WebmachineCallback<bool>,
  /// HTTP methods that are known to the resource. Default includes all standard HTTP methods.
  /// One could override this to allow additional methods
  pub known_methods: Vec<String>,
  /// If the URI is too long to be processed, this should return true, which will result in a
  /// '414 Request URI Too Long' response. Defaults to false.
  pub uri_too_long: WebmachineCallback<bool>,
  /// HTTP methods that are allowed on this resource. Defaults to GET','HEAD and 'OPTIONS'.
  pub allowed_methods: Vec<String>,
  /// If the request is malformed, this should return true, which will result in a
  /// '400 Malformed Request' response. Defaults to false.
  pub malformed_request: WebmachineCallback<bool>,
  /// Is the client or request not authorized? Returning a Some<String>
  /// will result in a '401 Unauthorized' response.  Defaults to None. If a Some(String) is
  /// returned, the string will be used as the value in the WWW-Authenticate header.
  pub not_authorized: WebmachineCallback<Option<String>>,
  /// Is the request or client forbidden? Returning true will result in a '403 Forbidden' response.
  /// Defaults to false.
  pub forbidden: WebmachineCallback<bool>,
  /// If the request includes any invalid Content-* headers, this should return true, which will
  /// result in a '501 Not Implemented' response. Defaults to false.
  pub unsupported_content_headers: WebmachineCallback<bool>,
  /// The list of acceptable content types. Defaults to 'application/json'. If the content type
  /// of the request is not in this list, a '415 Unsupported Media Type' response is returned.
  pub acceptable_content_types: Vec<String>,
  /// If the entity length on PUT or POST is invalid, this should return false, which will result
  /// in a '413 Request Entity Too Large' response. Defaults to true.
  pub valid_entity_length: WebmachineCallback<bool>,
  /// This is called just before the final response is constructed and sent. This allows the
  /// response to be modified. The default implementation adds CORS headers to the response
  pub finish_request: WebmachineCallback<()>,
  /// If the OPTIONS method is supported and is used, this returns a HashMap of headers that
  /// should appear in the response. Defaults to CORS headers.
  pub options: WebmachineCallback<Option<HashMap<String, Vec<String>>>>,
  /// The list of content types that this resource produces. Defaults to 'application/json'. If
  /// more than one is provided, and the client does not supply an Accept header, the first one
  /// will be selected.
  pub produces: Vec<String>,
  /// The list of content languages that this resource provides. Defaults to an empty list,
  /// which represents all languages. If more than one is provided, and the client does not
  /// supply an Accept-Language header, the first one will be selected.
  pub languages_provided: Vec<String>,
  /// The list of charsets that this resource provides. Defaults to an empty list,
  /// which represents all charsets with ISO-8859-1 as the default. If more than one is provided,
  /// and the client does not supply an Accept-Charset header, the first one will be selected.
  pub charsets_provided: Vec<String>,
  /// The list of encodings your resource wants to provide. The encoding will be applied to the
  /// response body automatically by Webmachine. Default includes only the 'identity' encoding.
  pub encodings_provided: Vec<String>,
  /// The list of header names that should be included in the response's Vary header. The standard
  /// content negotiation headers (Accept, Accept-Encoding, Accept-Charset, Accept-Language) do
  /// not need to be specified here as Webmachine will add the correct elements of those
  /// automatically depending on resource behavior. Default is an empty list.
  pub variances: Vec<String>,
  /// Does the resource exist? Returning a false value will result in a '404 Not Found' response
  /// unless it is a PUT or POST. Defaults to true.
  pub resource_exists: WebmachineCallback<bool>,
  /// If this resource is known to have existed previously, this should return true. Default is false.
  pub previously_existed: WebmachineCallback<bool>,
  /// If this resource has moved to a new location permanently, this should return the new
  /// location as a String. Default is to return None
  pub moved_permanently: WebmachineCallback<Option<String>>,
  /// If this resource has moved to a new location temporarily, this should return the new
  /// location as a String. Default is to return None
  pub moved_temporarily: WebmachineCallback<Option<String>>,
  /// If this returns true, the client will receive a '409 Conflict' response. This is only
  /// called for PUT requests. Default is false.
  pub is_conflict: WebmachineCallback<bool>,
  /// Return true if the resource accepts POST requests to nonexistent resources. Defaults to false.
  pub allow_missing_post: WebmachineCallback<bool>,
  /// If this returns a value, it will be used as the value of the ETag header and for
  /// comparison in conditional requests. Default is None.
  pub generate_etag: WebmachineCallback<Option<String>>,
  /// Returns the last modified date and time of the resource which will be added as the
  /// Last-Modified header in the response and used in negotiating conditional requests.
  /// Default is None
  pub last_modified: WebmachineCallback<Option<DateTime<FixedOffset>>>,
  /// Called when a DELETE request should be enacted. Return `Ok(true)` if the deletion succeeded,
  /// and `Ok(false)` if the deletion was accepted but cannot yet be guaranteed to have finished.
  /// If the delete fails for any reason, return an Err with the status code you wish returned
  /// (a 500 status makes sense).
  /// Defaults to `Ok(true)`.
  pub delete_resource: WebmachineCallback<Result<bool, u16>>,
  /// If POST requests should be treated as a request to put content into a (potentially new)
  /// resource as opposed to a generic submission for processing, then this should return true.
  /// If it does return true, then `create_path` will be called and the rest of the request will
  /// be treated much like a PUT to the path returned by that call. Default is false.
  pub post_is_create: WebmachineCallback<bool>,
  /// If `post_is_create` returns false, then this will be called to process any POST request.
  /// If it succeeds, return `Ok(true)`, `Ok(false)` otherwise. If it fails for any reason,
  /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
  /// Default is false. If you want the result of processing the POST to be a redirect, set
  /// `context.redirect` to true.
  pub process_post: WebmachineCallback<Result<bool, u16>>,
  /// This will be called on a POST request if `post_is_create` returns true. It should create
  /// the new resource and return the path as a valid URI part following the dispatcher prefix.
  /// That path will replace the previous one in the return value of `WebmachineRequest.request_path`
  /// for all subsequent resource function calls in the course of this request and will be set
  /// as the value of the Location header of the response. If it fails for any reason,
  /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
  /// Default will return an `Ok(WebmachineRequest.request_path)`. If you want the result of
  /// processing the POST to be a redirect, set `context.redirect` to true.
  pub create_path: WebmachineCallback<Result<String, u16>>,
  /// This will be called to process any PUT request. If it succeeds, return `Ok(true)`,
  /// `Ok(false)` otherwise. If it fails for any reason, return an Err with the status code
  /// you wish returned (e.g., a 500 status makes sense). Default is `Ok(true)`
  pub process_put: WebmachineCallback<Result<bool, u16>>,
  /// If this returns true, then it is assumed that multiple representations of the response are
  /// possible and a single one cannot be automatically chosen, so a 300 Multiple Choices will
  /// be sent instead of a 200. Default is false.
  pub multiple_choices: WebmachineCallback<bool>,
  /// If the resource expires, this should return the date/time it expires. Default is None.
  pub expires: WebmachineCallback<Option<DateTime<FixedOffset>>>
}

fn true_fn(_: &mut WebmachineContext, _: &WebmachineResource) -> bool {
  true
}

fn false_fn(_: &mut WebmachineContext, _: &WebmachineResource) -> bool {
  false
}

fn none_fn<T>(_: &mut WebmachineContext, _: &WebmachineResource) -> Option<T> {
  None
}

impl Default for WebmachineResource {
  fn default() -> WebmachineResource {
    WebmachineResource {
      finalise_response: None,
      available: callback(true_fn),
      known_methods: owned_vec(&["OPTIONS", "GET", "POST", "PUT", "DELETE", "HEAD", "TRACE", "CONNECT", "PATCH"]),
      uri_too_long: callback(false_fn),
      allowed_methods: owned_vec(&["OPTIONS", "GET", "HEAD"]),
      malformed_request: callback(false_fn),
      not_authorized: callback(none_fn),
      forbidden: callback(false_fn),
      unsupported_content_headers: callback(false_fn),
      acceptable_content_types: owned_vec(&["application/json"]),
      valid_entity_length: callback(true_fn),
      finish_request: callback(|context, resource| context.response.add_cors_headers(&resource.allowed_methods)),
      options: callback(|_, resource| Some(WebmachineResponse::cors_headers(&resource.allowed_methods))),
      produces: vec!["application/json".to_string()],
      languages_provided: Vec::new(),
      charsets_provided: Vec::new(),
      encodings_provided: vec!["identity".to_string()],
      variances: Vec::new(),
      resource_exists: callback(true_fn),
      previously_existed: callback(false_fn),
      moved_permanently: callback(none_fn),
      moved_temporarily: callback(none_fn),
      is_conflict: callback(false_fn),
      allow_missing_post: callback(false_fn),
      generate_etag: callback(none_fn),
      last_modified: callback(none_fn),
      delete_resource: callback(|_, _| Ok(true)),
      post_is_create: callback(false_fn),
      process_post: callback(|_, _| Ok(false)),
      process_put: callback(|_, _| Ok(true)),
      multiple_choices: callback(false_fn),
      create_path: callback(|context, _| Ok(context.request.request_path.clone())),
      expires: callback(none_fn),
      render_response: callback(none_fn)
    }
  }
}

fn sanitise_path(path: &str) -> Vec<String> {
  path.split("/").filter(|p| !p.is_empty()).map(|p| p.to_string()).collect()
}

fn join_paths(base: &Vec<String>, path: &Vec<String>) -> String {
  let mut paths = base.clone();
  paths.extend_from_slice(path);
  let filtered: Vec<String> = paths.iter().cloned().filter(|p| !p.is_empty()).collect();
  if filtered.is_empty() {
    "/".to_string()
  } else {
    let new_path = filtered.iter().join("/");
    if new_path.starts_with("/") {
      new_path
    } else {
      "/".to_owned() + &new_path
    }
  }
}

const MAX_STATE_MACHINE_TRANSITIONS: u8 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Decision {
    Start,
    End(u16),
    A3Options,
    B3Options,
    B4RequestEntityTooLarge,
    B5UnknownContentType,
    B6UnsupportedContentHeader,
    B7Forbidden,
    B8Authorized,
    B9MalformedRequest,
    B10MethodAllowed,
    B11UriTooLong,
    B12KnownMethod,
    B13Available,
    C3AcceptExists,
    C4AcceptableMediaTypeAvailable,
    D4AcceptLanguageExists,
    D5AcceptableLanguageAvailable,
    E5AcceptCharsetExists,
    E6AcceptableCharsetAvailable,
    F6AcceptEncodingExists,
    F7AcceptableEncodingAvailable,
    G7ResourceExists,
    G8IfMatchExists,
    G9IfMatchStarExists,
    G11EtagInIfMatch,
    H7IfMatchStarExists,
    H10IfUnmodifiedSinceExists,
    H11IfUnmodifiedSinceValid,
    H12LastModifiedGreaterThanUMS,
    I4HasMovedPermanently,
    I12IfNoneMatchExists,
    I13IfNoneMatchStarExists,
    I7Put,
    J18GetHead,
    K5HasMovedPermanently,
    K7ResourcePreviouslyExisted,
    K13ETagInIfNoneMatch,
    L5HasMovedTemporarily,
    L7Post,
    L13IfModifiedSinceExists,
    L14IfModifiedSinceValid,
    L15IfModifiedSinceGreaterThanNow,
    L17IfLastModifiedGreaterThanMS,
    M5Post,
    M7PostToMissingResource,
    M16Delete,
    M20DeleteEnacted,
    N5PostToMissingResource,
    N11Redirect,
    N16Post,
    O14Conflict,
    O16Put,
    O18MultipleRepresentations,
    O20ResponseHasBody,
    P3Conflict,
    P11NewResource
}

impl Decision {
    fn is_terminal(&self) -> bool {
        match self {
            &Decision::End(_) => true,
            &Decision::A3Options => true,
            _ => false
        }
    }
}

enum Transition {
  To(Decision),
  Branch(Decision, Decision)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DecisionResult {
  True(String),
  False(String),
  StatusCode(u16)
}

impl DecisionResult {
  fn wrap(result: bool, reason: &str) -> DecisionResult {
    if result {
      DecisionResult::True(format!("is: {}", reason))
    } else {
      DecisionResult::False(format!("is not: {}", reason))
    }
  }
}

lazy_static! {
    static ref TRANSITION_MAP: HashMap<Decision, Transition> = hashmap!{
        Decision::Start => Transition::To(Decision::B13Available),
        Decision::B3Options => Transition::Branch(Decision::A3Options, Decision::C3AcceptExists),
        Decision::B4RequestEntityTooLarge => Transition::Branch(Decision::End(413), Decision::B3Options),
        Decision::B5UnknownContentType => Transition::Branch(Decision::End(415), Decision::B4RequestEntityTooLarge),
        Decision::B6UnsupportedContentHeader => Transition::Branch(Decision::End(501), Decision::B5UnknownContentType),
        Decision::B7Forbidden => Transition::Branch(Decision::End(403), Decision::B6UnsupportedContentHeader),
        Decision::B8Authorized => Transition::Branch(Decision::B7Forbidden, Decision::End(401)),
        Decision::B9MalformedRequest => Transition::Branch(Decision::End(400), Decision::B8Authorized),
        Decision::B10MethodAllowed => Transition::Branch(Decision::B9MalformedRequest, Decision::End(405)),
        Decision::B11UriTooLong => Transition::Branch(Decision::End(414), Decision::B10MethodAllowed),
        Decision::B12KnownMethod => Transition::Branch(Decision::B11UriTooLong, Decision::End(501)),
        Decision::B13Available => Transition::Branch(Decision::B12KnownMethod, Decision::End(503)),
        Decision::C3AcceptExists => Transition::Branch(Decision::C4AcceptableMediaTypeAvailable, Decision::D4AcceptLanguageExists),
        Decision::C4AcceptableMediaTypeAvailable => Transition::Branch(Decision::D4AcceptLanguageExists, Decision::End(406)),
        Decision::D4AcceptLanguageExists => Transition::Branch(Decision::D5AcceptableLanguageAvailable, Decision::E5AcceptCharsetExists),
        Decision::D5AcceptableLanguageAvailable => Transition::Branch(Decision::E5AcceptCharsetExists, Decision::End(406)),
        Decision::E5AcceptCharsetExists => Transition::Branch(Decision::E6AcceptableCharsetAvailable, Decision::F6AcceptEncodingExists),
        Decision::E6AcceptableCharsetAvailable => Transition::Branch(Decision::F6AcceptEncodingExists, Decision::End(406)),
        Decision::F6AcceptEncodingExists => Transition::Branch(Decision::F7AcceptableEncodingAvailable, Decision::G7ResourceExists),
        Decision::F7AcceptableEncodingAvailable => Transition::Branch(Decision::G7ResourceExists, Decision::End(406)),
        Decision::G7ResourceExists => Transition::Branch(Decision::G8IfMatchExists, Decision::H7IfMatchStarExists),
        Decision::G8IfMatchExists => Transition::Branch(Decision::G9IfMatchStarExists, Decision::H10IfUnmodifiedSinceExists),
        Decision::G9IfMatchStarExists => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::G11EtagInIfMatch),
        Decision::G11EtagInIfMatch => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::End(412)),
        Decision::H7IfMatchStarExists => Transition::Branch(Decision::End(412), Decision::I7Put),
        Decision::H10IfUnmodifiedSinceExists => Transition::Branch(Decision::H11IfUnmodifiedSinceValid, Decision::I12IfNoneMatchExists),
        Decision::H11IfUnmodifiedSinceValid => Transition::Branch(Decision::H12LastModifiedGreaterThanUMS, Decision::I12IfNoneMatchExists),
        Decision::H12LastModifiedGreaterThanUMS => Transition::Branch(Decision::End(412), Decision::I12IfNoneMatchExists),
        Decision::I4HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::P3Conflict),
        Decision::I7Put => Transition::Branch(Decision::I4HasMovedPermanently, Decision::K7ResourcePreviouslyExisted),
        Decision::I12IfNoneMatchExists => Transition::Branch(Decision::I13IfNoneMatchStarExists, Decision::L13IfModifiedSinceExists),
        Decision::I13IfNoneMatchStarExists => Transition::Branch(Decision::J18GetHead, Decision::K13ETagInIfNoneMatch),
        Decision::J18GetHead => Transition::Branch(Decision::End(304), Decision::End(412)),
        Decision::K13ETagInIfNoneMatch => Transition::Branch(Decision::J18GetHead, Decision::L13IfModifiedSinceExists),
        Decision::K5HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::L5HasMovedTemporarily),
        Decision::K7ResourcePreviouslyExisted => Transition::Branch(Decision::K5HasMovedPermanently, Decision::L7Post),
        Decision::L5HasMovedTemporarily => Transition::Branch(Decision::End(307), Decision::M5Post),
        Decision::L7Post => Transition::Branch(Decision::M7PostToMissingResource, Decision::End(404)),
        Decision::L13IfModifiedSinceExists => Transition::Branch(Decision::L14IfModifiedSinceValid, Decision::M16Delete),
        Decision::L14IfModifiedSinceValid => Transition::Branch(Decision::L15IfModifiedSinceGreaterThanNow, Decision::M16Delete),
        Decision::L15IfModifiedSinceGreaterThanNow => Transition::Branch(Decision::M16Delete, Decision::L17IfLastModifiedGreaterThanMS),
        Decision::L17IfLastModifiedGreaterThanMS => Transition::Branch(Decision::M16Delete, Decision::End(304)),
        Decision::M5Post => Transition::Branch(Decision::N5PostToMissingResource, Decision::End(410)),
        Decision::M7PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(404)),
        Decision::M16Delete => Transition::Branch(Decision::M20DeleteEnacted, Decision::N16Post),
        Decision::M20DeleteEnacted => Transition::Branch(Decision::O20ResponseHasBody, Decision::End(202)),
        Decision::N5PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(410)),
        Decision::N11Redirect => Transition::Branch(Decision::End(303), Decision::P11NewResource),
        Decision::N16Post => Transition::Branch(Decision::N11Redirect, Decision::O16Put),
        Decision::O14Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::O16Put => Transition::Branch(Decision::O14Conflict, Decision::O18MultipleRepresentations),
        Decision::P3Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::P11NewResource => Transition::Branch(Decision::End(201), Decision::O20ResponseHasBody),
        Decision::O18MultipleRepresentations => Transition::Branch(Decision::End(300), Decision::End(200)),
        Decision::O20ResponseHasBody => Transition::Branch(Decision::O18MultipleRepresentations, Decision::End(204))
    };
}

fn resource_etag_matches_header_values(
  resource: &WebmachineResource,
  context: &mut WebmachineContext,
  header: &str
) -> bool {
  let header_values = context.request.find_header(header);
  match (resource.generate_etag)(context, resource) {
    Some(etag) => {
      header_values.iter().find(|val| {
        if val.value.starts_with("W/") {
          val.weak_etag().unwrap() == etag
        } else {
          val.value == etag
        }
      }).is_some()
    },
    None => false
  }
}

fn validate_header_date(
  request: &WebmachineRequest,
  header: &str,
  context_meta: &mut Option<DateTime<FixedOffset>>
) -> bool {
  let header_values = request.find_header(header);
  if let Some(date_value) = header_values.first() {
    match DateTime::parse_from_rfc2822(&date_value.value) {
      Ok(datetime) => {
        *context_meta = Some(datetime.clone());
        true
      },
      Err(err) => {
        debug!("Failed to parse '{}' header value '{:?}' - {}", header, date_value, err);
        false
      }
    }
  } else {
    false
  }
}

fn execute_decision(
  decision: &Decision,
  context: &mut WebmachineContext,
  resource: &WebmachineResource
) -> DecisionResult {
  match decision {
    Decision::B10MethodAllowed => {
      match resource.allowed_methods
        .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()) {
        Some(_) => DecisionResult::True("method is in the list of allowed methods".to_string()),
        None => {
          context.response.add_header("Allow", resource.allowed_methods
            .iter()
            .cloned()
            .map(HeaderValue::basic)
            .collect());
          DecisionResult::False("method is not in the list of allowed methods".to_string())
        }
      }
    },
    Decision::B11UriTooLong => {
      DecisionResult::wrap((resource.uri_too_long)(context, resource), "URI too long")
    },
    Decision::B12KnownMethod => DecisionResult::wrap(resource.known_methods
      .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()).is_some(),
      "known method"),
    Decision::B13Available => {
      DecisionResult::wrap((resource.available)(context, resource), "available")
    },
    Decision::B9MalformedRequest => {
      DecisionResult::wrap((resource.malformed_request)(context, resource), "malformed request")
    },
    Decision::B8Authorized => {
      match (resource.not_authorized)(context, resource) {
        Some(realm) => {
          context.response.add_header("WWW-Authenticate", vec![HeaderValue::parse_string(realm.as_str())]);
          DecisionResult::False("is not authorized".to_string())
        },
        None => DecisionResult::True("is not authorized".to_string())
      }
    },
    Decision::B7Forbidden => {
      DecisionResult::wrap((resource.forbidden)(context, resource), "forbidden")
    },
    Decision::B6UnsupportedContentHeader => {
      DecisionResult::wrap((resource.unsupported_content_headers)(context, resource), "unsupported content headers")
    },
    Decision::B5UnknownContentType => {
      DecisionResult::wrap(context.request.is_put_or_post() && resource.acceptable_content_types
        .iter().find(|ct| context.request.content_type().to_uppercase() == ct.to_uppercase() )
        .is_none(), "acceptable content types")
    },
    Decision::B4RequestEntityTooLarge => {
      DecisionResult::wrap(context.request.is_put_or_post() && !(resource.valid_entity_length)(context, resource),
        "valid entity length")
    },
    Decision::B3Options => DecisionResult::wrap(context.request.is_options(), "options"),
    Decision::C3AcceptExists => DecisionResult::wrap(context.request.has_accept_header(), "has accept header"),
    Decision::C4AcceptableMediaTypeAvailable => match content_negotiation::matching_content_type(resource, &context.request) {
      Some(media_type) => {
        context.selected_media_type = Some(media_type);
        DecisionResult::True("acceptable media type is available".to_string())
      },
      None => DecisionResult::False("acceptable media type is not available".to_string())
    },
    Decision::D4AcceptLanguageExists => DecisionResult::wrap(context.request.has_accept_language_header(),
                                                             "has accept language header"),
    Decision::D5AcceptableLanguageAvailable => match content_negotiation::matching_language(resource, &context.request) {
      Some(language) => {
        if language != "*" {
          context.selected_language = Some(language.clone());
          context.response.add_header("Content-Language", vec![HeaderValue::parse_string(&language)]);
        }
        DecisionResult::True("acceptable language is available".to_string())
      },
      None => DecisionResult::False("acceptable language is not available".to_string())
    },
    Decision::E5AcceptCharsetExists => DecisionResult::wrap(context.request.has_accept_charset_header(),
                                                            "accept charset exists"),
    Decision::E6AcceptableCharsetAvailable => match content_negotiation::matching_charset(resource, &context.request) {
      Some(charset) => {
        if charset != "*" {
            context.selected_charset = Some(charset.clone());
        }
        DecisionResult::True("acceptable charset is available".to_string())
      },
      None => DecisionResult::False("acceptable charset is not available".to_string())
    },
    Decision::F6AcceptEncodingExists => DecisionResult::wrap(context.request.has_accept_encoding_header(),
                                                             "accept encoding exists"),
    Decision::F7AcceptableEncodingAvailable => match content_negotiation::matching_encoding(resource, &context.request) {
      Some(encoding) => {
        context.selected_encoding = Some(encoding.clone());
        if encoding != "identity" {
            context.response.add_header("Content-Encoding", vec![HeaderValue::parse_string(&encoding)]);
        }
        DecisionResult::True("acceptable encoding is available".to_string())
      },
      None => DecisionResult::False("acceptable encoding is not available".to_string())
    },
    Decision::G7ResourceExists => {
      DecisionResult::wrap((resource.resource_exists)(context, resource), "resource exists")
    },
    Decision::G8IfMatchExists => DecisionResult::wrap(context.request.has_header("If-Match"),
                                                      "match exists"),
    Decision::G9IfMatchStarExists | &Decision::H7IfMatchStarExists => DecisionResult::wrap(
        context.request.has_header_value("If-Match", "*"), "match star exists"),
    Decision::G11EtagInIfMatch => DecisionResult::wrap(resource_etag_matches_header_values(resource, context, "If-Match"),
                                                       "etag in if match"),
    Decision::H10IfUnmodifiedSinceExists => DecisionResult::wrap(context.request.has_header("If-Unmodified-Since"),
                                                                 "unmodified since exists"),
    Decision::H11IfUnmodifiedSinceValid => DecisionResult::wrap(validate_header_date(&context.request, "If-Unmodified-Since", &mut context.if_unmodified_since),
                                                                "unmodified since valid"),
    Decision::H12LastModifiedGreaterThanUMS => {
      match context.if_unmodified_since {
        Some(unmodified_since) => {
          match (resource.last_modified)(context, resource) {
            Some(datetime) => DecisionResult::wrap(datetime > unmodified_since,
                                                   "resource last modified date is greater than unmodified since"),
            None => DecisionResult::False("resource has no last modified date".to_string())
          }
        },
        None => DecisionResult::False("resource does not provide last modified date".to_string())
      }
    },
    Decision::I7Put => if context.request.is_put() {
      context.new_resource = true;
      DecisionResult::True("is a PUT request".to_string())
    } else {
      DecisionResult::False("is not a PUT request".to_string())
    },
    Decision::I12IfNoneMatchExists => DecisionResult::wrap(context.request.has_header("If-None-Match"),
                                                           "none match exists"),
    Decision::I13IfNoneMatchStarExists => DecisionResult::wrap(context.request.has_header_value("If-None-Match", "*"),
                                                               "none match star exists"),
    Decision::J18GetHead => DecisionResult::wrap(context.request.is_get_or_head(),
                                                 "is GET or HEAD request"),
    Decision::K7ResourcePreviouslyExisted => {
      DecisionResult::wrap((resource.previously_existed)(context, resource), "resource previously existed")
    },
    Decision::K13ETagInIfNoneMatch => DecisionResult::wrap(resource_etag_matches_header_values(resource, context, "If-None-Match"),
                                                           "ETag in if none match"),
    Decision::L5HasMovedTemporarily => {
      match (resource.moved_temporarily)(context, resource) {
        Some(location) => {
          context.response.add_header("Location", vec![HeaderValue::basic(&location)]);
          DecisionResult::True("resource has moved temporarily".to_string())
        },
        None => DecisionResult::False("resource has not moved temporarily".to_string())
      }
    },
    Decision::L7Post | &Decision::M5Post | &Decision::N16Post => DecisionResult::wrap(context.request.is_post(),
                                                                                      "a POST request"),
    Decision::L13IfModifiedSinceExists => DecisionResult::wrap(context.request.has_header("If-Modified-Since"),
                                                               "if modified since exists"),
    Decision::L14IfModifiedSinceValid => DecisionResult::wrap(validate_header_date(&context.request,
        "If-Modified-Since", &mut context.if_modified_since), "modified since valid"),
    Decision::L15IfModifiedSinceGreaterThanNow => {
        let datetime = context.if_modified_since.unwrap();
        let timezone = datetime.timezone();
        DecisionResult::wrap(datetime > Utc::now().with_timezone(&timezone),
                             "modified since greater than now")
    },
    Decision::L17IfLastModifiedGreaterThanMS => {
      match context.if_modified_since {
        Some(unmodified_since) => {
          match (resource.last_modified)(context, resource) {
            Some(datetime) => DecisionResult::wrap(datetime > unmodified_since,
                                                   "last modified greater than modified since"),
            None => DecisionResult::False("resource has no last modified date".to_string())
          }
        },
        None => DecisionResult::False("resource does not return if_modified_since".to_string())
      }
    },
    Decision::I4HasMovedPermanently | &Decision::K5HasMovedPermanently => {
      match (resource.moved_permanently)(context, resource) {
        Some(location) => {
          context.response.add_header("Location", vec![HeaderValue::basic(&location)]);
          DecisionResult::True("resource has moved permanently".to_string())
        },
        None => DecisionResult::False("resource has not moved permanently".to_string())
      }
    },
    Decision::M7PostToMissingResource | &Decision::N5PostToMissingResource => {
      if (resource.allow_missing_post)(context, resource) {
        context.new_resource = true;
        DecisionResult::True("resource allows POST to missing resource".to_string())
      } else {
        DecisionResult::False("resource does not allow POST to missing resource".to_string())
      }
    },
    Decision::M16Delete => DecisionResult::wrap(context.request.is_delete(),
                                                "a DELETE request"),
    Decision::M20DeleteEnacted => {
      match (resource.delete_resource)(context, resource) {
        Ok(result) => DecisionResult::wrap(result, "resource DELETE succeeded"),
        Err(status) => DecisionResult::StatusCode(status)
      }
    },
    Decision::N11Redirect => {
      if (resource.post_is_create)(context, resource) {
        match (resource.create_path)(context, resource) {
          Ok(path) => {
            let base_path = sanitise_path(&context.request.base_path);
            let new_path = join_paths(&base_path, &sanitise_path(&path));
            context.request.request_path = path.clone();
            context.response.add_header("Location", vec![HeaderValue::basic(&new_path)]);
            DecisionResult::wrap(context.redirect, "should redirect")
          },
          Err(status) => DecisionResult::StatusCode(status)
        }
      } else {
        match (resource.process_post)(context, resource) {
          Ok(_) => DecisionResult::wrap(context.redirect, "processing POST succeeded"),
          Err(status) => DecisionResult::StatusCode(status)
        }
      }
    },
    Decision::P3Conflict | &Decision::O14Conflict => {
      DecisionResult::wrap((resource.is_conflict)(context, resource), "resource conflict")
    },
    Decision::P11NewResource => {
      if context.request.is_put() {
        match (resource.process_put)(context, resource) {
          Ok(_) => DecisionResult::wrap(context.new_resource, "process PUT succeeded"),
          Err(status) => DecisionResult::StatusCode(status)
        }
      } else {
        DecisionResult::wrap(context.new_resource, "new resource creation succeeded")
      }
    },
    Decision::O16Put => DecisionResult::wrap(context.request.is_put(), "a PUT request"),
    Decision::O18MultipleRepresentations => {
      DecisionResult::wrap((resource.multiple_choices)(context, resource), "multiple choices exist")
    },
    Decision::O20ResponseHasBody => DecisionResult::wrap(context.response.has_body(), "response has a body"),
    _ => DecisionResult::False("default decision is false".to_string())
  }
}

fn execute_state_machine(context: &mut WebmachineContext, resource: &WebmachineResource) {
  let mut state = Decision::Start;
  let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
  let mut loop_count = 0;
  while !state.is_terminal() {
    loop_count += 1;
    if loop_count >= MAX_STATE_MACHINE_TRANSITIONS {
      panic!("State machine has not terminated within {} transitions!", loop_count);
    }
    trace!("state is {:?}", state);
    state = match TRANSITION_MAP.get(&state) {
      Some(transition) => match transition {
        Transition::To(decision) => {
          trace!("Transitioning to {:?}", decision);
          decision.clone()
        },
        Transition::Branch(decision_true, decision_false) => {
          match execute_decision(&state, context, resource) {
            DecisionResult::True(reason) => {
              trace!("Transitioning from {:?} to {:?} as decision is true -> {}", state, decision_true, reason);
              decisions.push((state, true, decision_true.clone()));
              decision_true.clone()
            },
            DecisionResult::False(reason) => {
              trace!("Transitioning from {:?} to {:?} as decision is false -> {}", state, decision_false, reason);
              decisions.push((state, false, decision_false.clone()));
              decision_false.clone()
            },
            DecisionResult::StatusCode(code) => {
              let decision = Decision::End(code);
              trace!("Transitioning from {:?} to {:?} as decision is a status code", state, decision);
              decisions.push((state, false, decision.clone()));
              decision.clone()
            }
          }
        }
      },
      None => {
        error!("Error transitioning from {:?}, the TRANSITION_MAP is mis-configured", state);
        decisions.push((state, false, Decision::End(500)));
        Decision::End(500)
      }
    }
  }
  trace!("Final state is {:?}", state);
  match state {
    Decision::End(status) => context.response.status = status,
    Decision::A3Options => {
      context.response.status = 204;
      match (resource.options)(context, resource) {
        Some(headers) => context.response.add_headers(headers),
        None => ()
      }
    },
    _ => ()
  }
}

fn update_paths_for_resource(request: &mut WebmachineRequest, base_path: &str) {
  request.base_path = base_path.into();
  if request.request_path.len() > base_path.len() {
    let request_path = request.request_path.clone();
    let subpath = request_path.split_at(base_path.len()).1;
    if subpath.starts_with("/") {
      request.request_path = subpath.to_string();
    } else {
      request.request_path = "/".to_owned() + subpath;
    }
  } else {
    request.request_path = "/".to_string();
  }
}

fn parse_header_values(value: &str) -> Vec<HeaderValue> {
  if value.is_empty() {
    Vec::new()
  } else {
    value.split(',').map(|s| HeaderValue::parse_string(s.trim())).collect()
  }
}

fn headers_from_http_request(headers: &HeaderMap<http::HeaderValue>) -> HashMap<String, Vec<HeaderValue>> {
  headers.iter()
    .map(|(name, value)| (name.to_string(), parse_header_values(value.to_str().unwrap_or_default())))
    .collect()
}

fn decode_query(query: &str) -> String {
  let mut chars = query.chars();
  let mut ch = chars.next();
  let mut result = String::new();

  while ch.is_some() {
    let c = ch.unwrap();
    if c == '%' {
      let c1 = chars.next();
      let c2 = chars.next();
      match (c1, c2) {
        (Some(v1), Some(v2)) => {
          let mut s = String::new();
          s.push(v1);
          s.push(v2);
          let decoded: Result<Vec<u8>, _> = hex::decode(s);
          match decoded {
            Ok(n) => result.push(n[0] as char),
            Err(_) => {
              result.push('%');
              result.push(v1);
              result.push(v2);
            }
          }
        },
        (Some(v1), None) => {
          result.push('%');
          result.push(v1);
        },
        _ => result.push('%')
      }
    } else if c == '+' {
      result.push(' ');
    } else {
      result.push(c);
    }

    ch = chars.next();
  }

  result
}

fn parse_query(query: &str) -> HashMap<String, Vec<String>> {
  if !query.is_empty() {
    query.split("&").map(|kv| {
      if kv.is_empty() {
        vec![]
      } else if kv.contains("=") {
        kv.splitn(2, "=").collect::<Vec<&str>>()
      } else {
        vec![kv]
      }
    }).fold(HashMap::new(), |mut map, name_value| {
      if !name_value.is_empty() {
        let name = decode_query(name_value[0]);
        let value = if name_value.len() > 1 {
          decode_query(name_value[1])
        } else {
          String::new()
        };
        map.entry(name).or_insert(vec![]).push(value);
      }
      map
    })
  } else {
    HashMap::new()
  }
}

async fn request_from_http_request(req: Request<Incoming>) -> WebmachineRequest {
  let request_path = req.uri().path().to_string();
  let method = req.method().to_string();
  let query = match req.uri().query() {
    Some(query) => parse_query(query),
    None => HashMap::new()
  };
  let headers = headers_from_http_request(req.headers());

  let body = match req.collect().await {
    Ok(body) => {
      let body = body.to_bytes();
      if body.is_empty() {
        None
      } else {
        Some(body.clone())
      }
    }
    Err(err) => {
      error!("Failed to read the request body: {}", err);
      None
    }
  };

  WebmachineRequest {
    request_path,
    base_path: "/".to_string(),
    method,
    headers,
    body,
    query
  }
}

fn finalise_response(context: &mut WebmachineContext, resource: &WebmachineResource) {
  if !context.response.has_header("Content-Type") {
    let media_type = match &context.selected_media_type {
      &Some(ref media_type) => media_type.clone(),
      &None => "application/json".to_string()
    };
    let charset = match &context.selected_charset {
      &Some(ref charset) => charset.clone(),
      &None => "ISO-8859-1".to_string()
    };
    let header = HeaderValue {
      value: media_type,
      params: hashmap!{ "charset".to_string() => charset },
      quote: false
    };
    context.response.add_header("Content-Type", vec![header]);
  }

  let mut vary_header = if !context.response.has_header("Vary") {
    resource.variances
      .iter()
      .map(|h| HeaderValue::parse_string(h))
      .collect()
  } else {
    Vec::new()
  };

  if resource.languages_provided.len() > 1 {
    vary_header.push(h!("Accept-Language"));
  }
  if resource.charsets_provided.len() > 1 {
    vary_header.push(h!("Accept-Charset"));
  }
  if resource.encodings_provided.len() > 1 {
    vary_header.push(h!("Accept-Encoding"));
  }
  if resource.produces.len() > 1 {
    vary_header.push(h!("Accept"));
  }

  if vary_header.len() > 1 {
    context.response.add_header("Vary", vary_header.iter().cloned().unique().collect());
  }

  if context.request.is_get_or_head() {
    {
      match (resource.generate_etag)(context, resource) {
        Some(etag) => context.response.add_header("ETag", vec![HeaderValue::basic(&etag).quote()]),
        None => ()
      }
    }
    {
      match (resource.expires)(context, resource) {
        Some(datetime) => context.response.add_header("Expires", vec![HeaderValue::basic(datetime.to_rfc2822()).quote()]),
        None => ()
      }
    }
    {
      match (resource.last_modified)(context, resource) {
        Some(datetime) => context.response.add_header("Last-Modified", vec![HeaderValue::basic(datetime.to_rfc2822()).quote()]),
        None => ()
      }
    }
  }

  if context.response.body.is_none() && context.response.status == 200 && context.request.is_get() {
    match (resource.render_response)(context, resource) {
      Some(body) => context.response.body = Some(body),
      None => ()
    }
  }

  match &resource.finalise_response {
    Some(callback) => {
      callback(context, resource);
    },
    None => ()
  }

  debug!("Final response: {:?}", context.response);
}

fn generate_http_response(context: &WebmachineContext) -> http::Result<Response<Full<Bytes>>> {
  let mut response = Response::builder().status(context.response.status);

  for (header, values) in context.response.headers.clone() {
    let header_values = values.iter().map(|h| h.to_string()).join(", ");
    response = response.header(&header, &header_values);
  }
  match context.response.body.clone() {
    Some(body) => response.body(Full::new(body.into())),
    None => response.body(Full::new(Bytes::default()))
  }
}

/// The main hyper dispatcher
pub struct WebmachineDispatcher {
  /// Map of routes to webmachine resources
  pub routes: BTreeMap<&'static str, WebmachineResource>
}

impl WebmachineDispatcher {
  /// Main dispatch function for the Webmachine. This will look for a matching resource
  /// based on the request path. If one is not found, a 404 Not Found response is returned
  pub async fn dispatch(&self, req: Request<Incoming>) -> http::Result<Response<Full<Bytes>>> {
    let mut context = self.context_from_http_request(req).await;
    self.dispatch_to_resource(&mut context);
    generate_http_response(&context)
  }

  async fn context_from_http_request(&self, req: Request<Incoming>) -> WebmachineContext {
    let request = request_from_http_request(req).await;
    WebmachineContext {
      request,
      response: WebmachineResponse::default(),
      .. WebmachineContext::default()
    }
  }

  fn match_paths(&self, request: &WebmachineRequest) -> Vec<String> {
    let request_path = sanitise_path(&request.request_path);
    self.routes
      .keys()
      .filter(|k| request_path.starts_with(&sanitise_path(k)))
      .map(|k| k.to_string())
      .collect()
  }

  fn lookup_resource(&self, path: &str) -> Option<&WebmachineResource> {
    self.routes.get(path)
  }

  /// Dispatches to the matching webmachine resource. If there is no matching resource, returns
  /// 404 Not Found response
  pub fn dispatch_to_resource(&self, context: &mut WebmachineContext) {
    let matching_paths = self.match_paths(&context.request);
    let ordered_by_length: Vec<String> = matching_paths.iter()
      .cloned()
      .sorted_by(|a, b| Ord::cmp(&b.len(), &a.len())).collect();
    match ordered_by_length.first() {
      Some(path) => {
        update_paths_for_resource(&mut context.request, path);
        if let Some(resource) = self.lookup_resource(path) {
          execute_state_machine(context, &resource);
          finalise_response(context, &resource);
        } else {
          context.response.status = 404;
        }
      },
      None => context.response.status = 404
    };
  }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod content_negotiation_tests;
