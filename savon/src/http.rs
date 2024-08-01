use crate::gen::{FromElement, ToElements};
use crate::rpser::{Method, Response};
use reqwest::header::{HeaderValue, COOKIE, SET_COOKIE};
use reqwest::{header, Client};
use std::fmt::Debug;

pub async fn one_way<Input: ToElements>(
    client: &Client,
    base_url: &str,
    ns: &str,
    method: &str,
    input: &Input,
) -> Result<(), crate::Error> {
    let mut v = input.to_elements();
    let mut m = Method::new(method);

    for el in v.drain(..) {
        m = m.with(el);
    }
    let s = m.as_xml(ns);
    trace!("sending: {}", s);

    let response: String = client
        .post(base_url)
        .header("Content-Type", "text/xml")
        .header("MessageType", "Call")
        .body(s)
        .send()
        .await?
        .text()
        .await?;

    trace!("received: {}", response);
    Ok(())
}

pub async fn request_response<Input: ToElements, Output: Debug + FromElement, Error>(
    client: &Client,
    base_url: &str,
    cookie:&mut Option<String>,
    ns: &str,
    method: &str,
    input: &Input,
) -> Result<Result<Output, Error>, crate::Error> {
    let mut v = input.to_elements();
    let mut m = Method::new(method);

    for el in v.drain(..) {
        m = m.with(el);
    }
    let s = m.as_xml(ns);
    trace!("sending: {}", s);

    let mut rb = client
        .post(base_url)
        .header("Content-Type", "application/soap+xml")
        .header("MessageType", "Call");
    if let Some(cookie) = cookie {
        rb = rb.header(COOKIE.as_str(), cookie.as_str());
    }

    let response = rb.body(s)
        .send()
        .await?;
    let headers = response.headers();
    trace!("headers: {:?}", headers);
    if let Some(header) = headers.get(SET_COOKIE.as_str()).map(|v|v.clone()){
        *cookie = Some(header.to_str()
            .map(|v|v.to_string())
            .map_err(|e|crate::Error::StringError(e.to_string()))?);
    }
    let body = response.text().await?;


    trace!("received: {}", body);
    let r = Response::from_xml(&body).unwrap();
    //trace!("parsed: {:#?}", r);
    let o = Output::from_element(&r.body);
    //trace!("output: {:#?}", o);

    o.map(Ok)
}
