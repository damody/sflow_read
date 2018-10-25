-- Your SQL goes here
CREATE TABLE flow (
    flow_id SERIAL NOT NULL,
    input_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    agent TEXT NOT NULL,
    utc INT NOT NULL,
    src TEXT NOT NULL,
    dst TEXT NOT NULL,
    srcport INT NOT NULL,
    dstport INT NOT NULL,
    ntype TEXT NOT NULL,
    size INT NOT NULL,
    PRIMARY Key(flow_id)
);
