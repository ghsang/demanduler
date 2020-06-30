extern crate daggy;

mod eventuler;

use std::io;
use std::sync::Mutex;

use actix_web::{middleware, web, App, Responder, HttpServer, HttpResponse, Either};
use chrono::{DateTime, Utc};
use serde::{Deserialize};

use eventuler::{Eventuler, Job};

async fn insert_job(
    eventuler: web::Data<Mutex<Eventuler>>,
    job: web::Json<Job>,
) -> impl Responder {
    let mut eventuler = eventuler.lock().unwrap();
    match (*eventuler).insert(job.into_inner()) {
        Err(e) => {
            let body = e.to_string();
            let response = HttpResponse::BadRequest().body(body);
            Either::A(response)
        },
        Ok(task) => Either::B(web::Json(task)),
    }
}

#[derive(Debug, Deserialize)]
struct Trigger {
    updatetime: DateTime<Utc>
}

async fn trigger(
    eventuler: web::Data<Mutex<Eventuler>>,
    name: web::Path<String>,
    trigger: web::Json<Trigger>
) -> impl Responder {
    let mut eventuler = eventuler.lock().unwrap();
    match (*eventuler).trigger(&name.into_inner(), trigger.updatetime) {
        Err(e) => {
            let body = e.to_string();
            let response = HttpResponse::BadRequest().body(body);
            Either::A(response)
        },
        Ok(tasks) => Either::B(web::Json(tasks)),
    }
}

async fn retrieve(
    eventuler: web::Data<Mutex<Eventuler>>,
) -> impl Responder {
    let eventuler = eventuler.lock().unwrap();
    web::Json(eventuler.graph())
}


#[actix_rt::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let eventuler = web::Data::new(Mutex::new(Eventuler::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(eventuler.clone())
            .wrap(middleware::Logger::default())
            .route("/jobs", web::post().to(insert_job))
            .route("/jobs/{name}", web::patch().to(trigger))
            .route("/jobs", web::get().to(retrieve))
    })
    .bind("127.0.0.1:3001")?
    .run()
    .await
}
