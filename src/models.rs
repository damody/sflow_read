use chrono;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub error: String,
}

#[derive(Serialize, Queryable, Debug)]
pub struct Flow {
    pub flow_id: i32,
    pub input_date: chrono::NaiveDateTime,
    pub agent: String,
    pub utc: i32,
    pub src: String,
    pub dst: String,
    pub srcport: i32,
    pub dstport: i32,
    pub ntype: String,
    pub size: i32,
}
