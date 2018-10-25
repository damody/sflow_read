table! {
    flow (flow_id) {
        flow_id -> Int4,
        input_date -> Timestamp,
        agent -> Text,
        utc -> Int4,
        src -> Text,
        dst -> Text,
        srcport -> Int4,
        dstport -> Int4,
        ntype -> Text,
        size -> Int4,
    }
}
