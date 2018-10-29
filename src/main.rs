#[link(name="openssl", kind="static")]
extern crate openssl;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[macro_use] 
extern crate text_io;
extern crate dotenv;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate regex;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate actix;
extern crate actix_web;
extern crate futures;
extern crate r2d2;
extern crate uuid;
extern crate bytes;
extern crate chrono;
extern crate r2d2_diesel;
#[macro_use]
extern crate diesel;
//use postgres::types::*;

use std::{thread};
mod flow;
mod sflow;
mod db;
mod schema;
mod models;
use flow::*;
use sflow::*;
use db::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager};
use actix::prelude::*;
use actix_web::{
    http, middleware, server, App,
    HttpRequest, HttpResponse, pred,
    http::{header},
    middleware::cors::Cors,
};

/// 404 handler
fn p404(_req: &HttpRequest<AppState>) -> actix_web::Result<actix_web::fs::NamedFile> {
    Ok(actix_web::fs::NamedFile::open("static/404.html")?.set_status_code(http::StatusCode::NOT_FOUND))
}


fn main() -> Result<(), Box<std::error::Error>> {
    ::std::env::set_var("RUST_LOG", "info");
    env_logger::init();
        
    let sys = actix::System::new("sflow-system");
    let manager = ConnectionManager::<PgConnection>::new(get_sql_url()?);
    let pool = r2d2::Pool::new(manager)
        .expect("Failed to create pool.");
    let poolc = pool.clone();
    let addr = SyncArbiter::start(8, move || DbExecutor(poolc.clone()));


    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file("rootAkey.pem", SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file("rootA.pem").unwrap();


    thread::spawn(move || {
        let conn: &PgConnection = &pool.clone().get().unwrap();
        match input_data(conn) {
            Ok(_x) => {},
            Err(x) => {
                info!("{}",x);
            }
        }
    });

    // Start http server
    server::new(move || {
        App::with_state(AppState{db: addr.clone()})
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(|app| {
                Cors::for_app(app)
                    .allowed_origin("https://10.71.5.59")
                    .allowed_methods(vec!["GET", "POST", "DELETE", "PUT"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600)
                    .resource("/flow", |r| {
                        r.post().with(flow_post);
                    })
                    .resource("/sflow", |r| {
                        r.post().with(flow_post);
                    })
                    .register()
            })
            .default_resource(|r| {
                    // 404 for GET request
                    r.method(http::Method::GET).f(p404);
                    // all requests that are not `GET`
                    r.route().filter(pred::Not(pred::Get())).f(
                        |_req| HttpResponse::MethodNotAllowed());
                })
    }).workers(8)
        .bind_ssl("10.73.16.244:8080", builder)
        .unwrap()
        .start();
    let _ = sys.run();

    Ok(())
}
