//! Db executor actor
use actix::prelude::*;
use actix_web::*;
use diesel;
use diesel::prelude::*;
use diesel::r2d2::{Pool, ConnectionManager};
use diesel::pg::PgConnection;
use schema;
use flow;
use chrono::{NaiveDateTime, NaiveDate};
use sflow::*;
use models;

no_arg_sql_function!(last_insert_id, diesel::sql_types::Integer);
pub type DBPool = Pool<ConnectionManager<PgConnection>>;

/// This is db executor actor. We are going to run 3 of them in parallel.
pub struct DbExecutor(pub DBPool);

/// State with DbExecutor address
pub struct AppState {
    pub db: Addr<DbExecutor>,
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct FlowD3 {
    nodes: Vec<FlowName>,
    links: Vec<FlowDirection2>,
}

impl Message for flow::FlowParams {
    type Result = Result<FlowD3, Error>;
}
impl Handler<flow::FlowParams> for DbExecutor {
    type Result = Result<FlowD3, Error>;

    fn handle(&mut self, msg: flow::FlowParams, _: &mut Self::Context) -> Self::Result {
        use self::schema::flow::dsl::*;
        info!("{:?}", msg);
        let up = NaiveDateTime::parse_from_str(&msg.up_date, "%Y-%m-%d %H:%M:%S").unwrap_or(
            NaiveDate::from_ymd(2014, 5, 17).and_hms(12, 34, 56));
        let dn = NaiveDateTime::parse_from_str(&msg.down_date, "%Y-%m-%d %H:%M:%S").unwrap_or(
            NaiveDate::from_ymd(2014, 5, 17).and_hms(12, 34, 56));
        let conn: &PgConnection = &self.0.get().unwrap();
        let loadflow = flow
            .filter(input_date.between(up, dn))
            .order(input_date.desc())
            .load::<models::Flow>(conn);
        if let Ok(loadflow) = loadflow {
            let fmap = build_graph_from_db(&loadflow);
            match fmap {
                Ok(x) => {
                    if let Ok((nodes_data, links_data)) = build_d3_data(&x) {
                        return Ok(FlowD3{nodes:nodes_data, links:links_data})
                    }
                },
                Err(x) => {
                    return Err(error::ErrorInternalServerError(x.to_string()))
                }
            }
        }
        Err(error::ErrorInternalServerError("Select Fail!"))
    }
}

