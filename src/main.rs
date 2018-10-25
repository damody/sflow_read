
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
// use postgres sql
extern crate postgres;
use postgres::{Connection};
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
    let sql = connect_sql();
    let conn: Connection = if sql.is_ok() { 
        info!("sql ok");
        sql.unwrap()
    } else {
        panic!("sql error");
    };
    
    thread::spawn(move || {
        match input_data(&conn) {
            Ok(_x) => {},
            Err(x) => {
                info!("{}",x);
            }
        }
    });
    let sys = actix::System::new("sflow-system");
    let manager = ConnectionManager::<PgConnection>::new(get_sql_url()?);
    let pool = r2d2::Pool::new(manager)
        .expect("Failed to create pool.");
    let addr = SyncArbiter::start(8, move || DbExecutor(pool.clone()));
    // Start http server
    server::new(move || {
        App::with_state(AppState{db: addr.clone()})
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(|app| {
                Cors::for_app(app)
                    .allowed_origin("http://localhost:4200")
                    .allowed_methods(vec!["GET", "POST", "DELETE", "PUT"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600)
                    .resource("/flow", |r| {
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
    }).workers(8).bind("127.0.0.1:8080")
        .unwrap()
        .start();
    let _ = sys.run();
    Ok(())
}
