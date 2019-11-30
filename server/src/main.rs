use futures::future;
use futures::Stream;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Client, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper_tls::HttpsConnector;

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn echo() -> impl Fn(Request<Body>) -> BoxFut {
    let https = HttpsConnector::new(4).expect("TLS initialization failed");
    let client = Client::builder().build::<_, hyper::Body>(https);
    let uri: http::Uri = "http://localhost:3001/".parse().unwrap();

    move |req: Request<Body>| -> BoxFut {
        let mut response = Response::new(Body::empty());

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                *response.body_mut() = Body::from("Try POSTing data to /echo");
            }
            (&Method::POST, "/echo") => {
                *response.body_mut() = req.into_body();
            }
            (&Method::POST, "/echo/uppercase") => {
                let mapping = req.into_body().map(|chunk| {
                    chunk
                        .iter()
                        .map(|byte| byte.to_ascii_uppercase())
                        .collect::<Vec<u8>>()
                });
                *response.body_mut() = Body::wrap_stream(mapping);
            }
            (&Method::POST, "/echo/reverse") => {
                let reversed = req.into_body().concat2().map(move |chunk| {
                    let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();

                    *response.body_mut() = Body::from(body);
                    response
                });
                return Box::new(reversed);
            }
            (&Method::GET, "/echo/vcloud") => {
                return Box::new(client.get(uri.clone()).then(|res| {
                    *response.body_mut() = match res {
                        Ok(res) => Body::wrap_stream(res.into_body()),
                        Err(err) => Body::from(format!("Error: {}", err)),
                    };
                    Ok(response)
                }))
            }
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        };

        Box::new(future::ok(response))
    }
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(echo()))
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}
