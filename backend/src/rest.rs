use std::sync::Mutex;

use tide::{StatusCode};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::eventuler::{Eventuler, Job};


#[derive(Debug, Serialize, Deserialize)]
struct Trigger {
    updatetime: DateTime<Utc>
}


async fn retrieve(
    req: tide::Request<Mutex<Eventuler>>
) -> tide::Result {
    let graph = req.state().lock().unwrap().graph();
    let mut res = tide::Response::new(200);
    res.set_body(tide::Body::from_json(&graph)?);
    Ok(res)
}


async fn insert_job(
    mut req: tide::Request<Mutex<Eventuler>>
) -> tide::Result {
    let job: Job = req.body_json().await?;
    let res = match req.state().lock().unwrap().insert(job) {
        Err(_) => {
            tide::Response::new(StatusCode::Conflict)
        },
        Ok(task) => {
            let mut res = tide::Response::new(StatusCode::Created);
            res.set_body(tide::Body::from_json(&task)?);
            res
        },
    };
    Ok(res)
}


async fn trigger(
    mut req: tide::Request<Mutex<Eventuler>>
) -> tide::Result {
    let name: String = req.param("name")?;
    let trigger: Trigger = req.body_json().await?;
    let res = match req.state().lock().unwrap().trigger(&name, trigger.updatetime) {
        Err(_) => tide::Response::new(StatusCode::Conflict),
        Ok(tasks) => {
            let mut res = tide::Response::new(StatusCode::Created);
            res.set_body(tide::Body::from_json(&tasks)?);
            res
        }
    };
    Ok(res)
}


pub async fn start_rest() -> Result<(), std::io::Error> {
    let eventuler = Mutex::new(Eventuler::new());
    let mut app = tide::with_state(eventuler);

    app.at("/jobs").get(retrieve);
    app.at("/jobs").post(insert_job);
    app.at("/jobs/:name").patch(trigger);

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
