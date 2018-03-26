use actix::*;
use actix_web::*;
use futures::future::Future;

use super::db::{DbExecutor, GetSources, GetTile};
use super::source::Sources;

pub struct State {
    db: Addr<Syn, DbExecutor>,
    sources: Sources,
}

fn index(req: HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.state()
        .db
        .send(GetSources {})
        .from_err()
        .and_then(|res| match res {
            Ok(sources) => Ok(httpcodes::HTTPOk.build().json(sources)?),
            Err(_) => Ok(httpcodes::HTTPInternalServerError.into()),
        })
        .responder()
}

fn source(req: HttpRequest<State>) -> Result<HttpResponse> {
    let sources = &req.state().sources;

    let source_id = req.match_info()
        .get("source")
        .ok_or(error::ErrorBadRequest("invalid source"))?;

    let source = sources.get(source_id).ok_or(error::ErrorNotFound(format!(
        "source {} not found",
        source_id
    )))?;

    Ok(httpcodes::HTTPOk.build().json(source)?)
}

fn tile(req: HttpRequest<State>) -> Result<Box<Future<Item = HttpResponse, Error = Error>>> {
    let sources = &req.state().sources;

    let source_id = req.match_info()
        .get("source")
        .ok_or(error::ErrorBadRequest("invalid source"))?;

    let source = sources.get(source_id).ok_or(error::ErrorNotFound(format!(
        "source {} not found",
        source_id
    )))?;

    let z = req.match_info()
        .get("z")
        .and_then(|i| i.parse::<u32>().ok())
        .ok_or(error::ErrorBadRequest("invalid z"))?;

    let x = req.match_info()
        .get("x")
        .and_then(|i| i.parse::<u32>().ok())
        .ok_or(error::ErrorBadRequest("invalid x"))?;

    let y = req.match_info()
        .get("y")
        .and_then(|i| i.parse::<u32>().ok())
        .ok_or(error::ErrorBadRequest("invalid y"))?;

    let condition = None;

    Ok(req.state()
        .db
        .send(GetTile {
            z: z,
            x: x,
            y: y,
            source: source.clone(),
            condition: condition,
        })
        .from_err()
        .and_then(|res| match res {
            Ok(tile) => Ok(HttpResponse::Ok()
                .content_type("application/x-protobuf")
                .body(tile)?),
            Err(_) => Ok(httpcodes::HTTPInternalServerError.into()),
        })
        .responder())
}

pub fn new(db_sync_arbiter: Addr<Syn, DbExecutor>, sources: Sources) -> Application<State> {
    let state = State {
        db: db_sync_arbiter,
        sources: sources,
    };

    let cors = middleware::cors::Cors::build()
        .finish()
        .expect("Can not create CORS middleware");

    Application::with_state(state)
        .middleware(middleware::Logger::default())
        .middleware(cors)
        .resource("/index.json", |r| r.method(Method::GET).a(index))
        .resource("/{source}.json", |r| r.method(Method::GET).f(source))
        .resource("/{source}/{z}/{x}/{y}.pbf", |r| {
            r.method(Method::GET).f(tile)
        })
}
