use std::{sync::{mpsc::{channel, Sender}, Arc, Mutex}, thread};

use actix_cors::Cors;
use actix_web::{get, post, put, web::{self, Json}, App, HttpResponse, HttpServer, Responder};
use pi_calc::{hub_runner, PiCalcSignal, PiCalcStatus, PiCalcUpdate};
use serde::Deserialize;

pub mod pi_calc;


#[get("health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("The pi runner is online")
}

#[get("hub_status")]
async fn hub_status(pi_status: web::Data<Arc<Mutex<PiCalcUpdate>>>) -> actix_web::Result<impl Responder>{
    let status = pi_status.lock().unwrap().clone();
    Ok(Json(status))
}

#[derive(Deserialize)]
struct TargetQuery{
    new: usize,
}

#[post("update_target")]
async fn update_target(info: web::Query<TargetQuery>,pi_status: web::Data<Arc<Mutex<PiCalcUpdate>>>) -> actix_web::Result<impl Responder>{
    pi_status.lock().unwrap().target = info.new as i128;
    Ok(HttpResponse::Ok())
}

#[post("start")]
async fn start_calculation(signal_channel: web::Data<Sender<PiCalcSignal>>) -> actix_web::Result<impl Responder> {
    let _ = signal_channel.send(PiCalcSignal::Start);
    Ok(HttpResponse::Ok())
}

#[actix_web::main]
async fn main()-> std::io::Result<()>{
    let pi_calc_status = PiCalcUpdate{spokes: vec![], status: PiCalcStatus::Init, progres: 0, last_20: None, duration: None, target: 100_000_000, chunk_size: 1024};
    let pi_calc_status = Arc::new(Mutex::new(pi_calc_status));

    let runner_arc = Arc::clone(&pi_calc_status);
    let (signal_tx,signal_rx) = channel();
    
    thread::spawn(|| hub_runner(runner_arc, signal_rx));

    let pi_calc_status = web::Data::new(pi_calc_status);
    let signal_tx = web::Data::new(signal_tx);

    HttpServer::new(move ||{
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:5500");
        App::new()
            .wrap(cors)
            .app_data(signal_tx.clone())
            .app_data(pi_calc_status.clone())
            .service(health)
            .service(hub_status)
            .service(update_target)
            .service(start_calculation)
    }).bind(("127.0.0.1", 8080))?
    .run()
    .await
}
