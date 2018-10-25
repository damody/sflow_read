use std::collections::HashMap;
use std::collections::HashSet;
use std::str;
use std::io;
use std;
use regex::Regex;
use postgres::{Connection, TlsMode};
use dotenv;
use models;

#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct Datagram {
    pub agent: String,
    pub samplev5: Vec<SampleV5>,
    pub datagramSourceIP: String,
    pub unixSecondsUTC: i32,
    pub datagramVersion: i8,
    pub sysUpTime: i64,
}

#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct SampleV5 {
    pub srcMAC: Option<String>,
    pub dstMAC: Option<String>,
    pub srcIP: Option<String>,
    pub dstIP: Option<String>,
    pub srcIP6: Option<String>,
    pub dstIP6: Option<String>,
    pub UDPSrcPort: Option<i32>,
    pub UDPDstPort: Option<i32>,
    pub TCPSrcPort: Option<i32>,
    pub TCPDstPort: Option<i32>,
    pub meanSkipCount: i32,
    pub inputPort: i8,
    pub outputPort: i8,
    pub sampleSequenceNo: i32,
    pub sampledPacketSize: i32,
}
#[derive(Debug, Default)]
pub struct FlowPoint {
    pub ip: String,
    pub input: i64,
    pub output: i64,
}
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct FlowDirection {
    pub agent: String,
    pub utc: i32,
    pub source: String,
    pub target: String,
    pub srcport: i32,
    pub dstport: i32,
    pub ntype: String,
    pub size: i32,
}
type FlowMap = HashMap<String, FlowDirection>;

#[allow(non_snake_case)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct FlowName {
    pub nodeId: i32,
    pub name: String,
}
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct FlowDirection2 {
    pub source: i32,
    pub target: i32,
    pub ntype: String,
    pub value: f32,
}

pub fn filter_multicast_ipv6(fm: &mut FlowMap) {
    let caps = Regex::new("^ff02:0000:.+").unwrap();
    let mut rms: Vec<String> = vec![];
    for fd in fm.iter() {
        if caps.is_match(&fd.1.source) || 
            caps.is_match(&fd.1.target) {
            rms.push(fd.0.clone());
        }
    }
    for n in rms.iter() {
        fm.remove(n);
    }
}
pub fn filter_local_ipv6(fm: &mut FlowMap) {
    let caps = Regex::new("^fe80:0000:.+").unwrap();
    let mut rms: Vec<String> = vec![];
    for fd in fm.iter() {
        if caps.is_match(&fd.1.source) || 
            caps.is_match(&fd.1.target) {
            rms.push(fd.0.clone());
        }
    }
    for n in rms.iter() {
        fm.remove(n);
    }
}
pub fn filter_ff_mac(fm: &mut FlowMap) {
    let caps = Regex::new("ffffffffffff").unwrap();
    let mut rms: Vec<String> = vec![];
    for fd in fm.iter() {
        if caps.is_match(&fd.1.source) || 
            caps.is_match(&fd.1.target) {
            rms.push(fd.0.clone());
        }
    }
    for n in rms.iter() {
        fm.remove(n);
    }
}

pub fn build_d3_data(fm: &FlowMap) -> Result<(Vec<FlowName>, Vec<FlowDirection2>), Box<std::error::Error>> {
    let mut pointset: HashSet<String> = HashSet::new();
    for fd in fm.iter() {
        pointset.insert(fd.1.source.clone());
        pointset.insert(fd.1.target.clone());
    }
    let mut points: Vec<FlowName> = vec![];
    let mut count = 0;
    for fd in pointset.iter() {
        points.push(FlowName {nodeId: count, name:fd.clone() });
        count += 1;
    }
    let mut pmap: HashMap<String, i32> = HashMap::new();
    for p in points.iter() {
        pmap.insert(p.name.clone(), p.nodeId);
    }
    let mut fmap: Vec<FlowDirection2> = vec![];
    for fd in fm.iter() {
        if fd.1.source != fd.1.target {
            fmap.push(FlowDirection2 {
                source: *pmap.get(&fd.1.source).unwrap(),
                target: *pmap.get(&fd.1.target).unwrap(),
                ntype: fd.1.ntype.clone(),
                value: fd.1.size as f32,
            });
        }
    }
    Ok( (points, fmap) )
}

#[allow(dead_code)]
pub fn build_d3_json(fm: &FlowMap) -> Result<(String, String), Box<std::error::Error>> {
    let (points, fmap) = build_d3_data(fm)?;
    Ok( (json!(points).to_string(), json!(fmap).to_string()) )
}

pub fn build_graph_from_db(data: &Vec<models::Flow>) -> Result<FlowMap, Box<std::error::Error>> {
    let mut fm: FlowMap = FlowMap::new();
    for s in data.iter() {
        let key = s.src.clone() + "=>" + &s.dst;
        let mut fd: Option<FlowDirection> = None;
        
        match fm.get_mut(&key) {
            None => {
                fd = Some(FlowDirection {
                    agent: s.agent.clone(),
                    utc: s.utc,
                    source: s.src.clone(),
                    target: s.dst.clone(),
                    srcport: s.srcport.clone(),
                    dstport: s.dstport.clone(),
                    ntype: s.ntype.clone(), 
                    size: s.size
                });
            },
            Some(ref mut x) => {
                x.size += s.size;
            }
        }
        if fd.is_some() {
            let fd = fd.unwrap();
            fm.insert(key, fd);
        }
    
    }
    Ok(fm)
}

pub fn build_graph(data: &Vec<Datagram>) -> Result<FlowMap, Box<std::error::Error>> {
    let mut fm: FlowMap = FlowMap::new();
    for dg in data.iter() {
        for s in dg.samplev5.iter() {
            let mut src:Option<String> = None;
            let mut dst:Option<String> = None;
            let mut srcport:Option<i32> = None;
            let mut dstport:Option<i32> = None;
            let size = s.sampledPacketSize;
            let mut ntype = "N/A";
            if s.srcIP.is_some() && s.dstIP.is_some() {
                src = s.srcIP.clone();
                dst = s.dstIP.clone();
                ntype = "ipv4";
            } else if s.srcIP6.is_some() && s.srcIP6.is_some() {
                src = s.srcIP6.clone();
                dst = s.dstIP6.clone();
                ntype = "ipv6";
            }
            if s.UDPSrcPort.is_some() && s.UDPDstPort.is_some() {
                srcport = s.UDPSrcPort.clone();
                dstport = s.UDPDstPort.clone();
            } else if s.TCPSrcPort.is_some() && s.TCPDstPort.is_some() {
                srcport = s.TCPSrcPort.clone();
                dstport = s.TCPDstPort.clone();
            }
            if src.is_none() {
                src = s.srcMAC.clone();
                dst = s.dstMAC.clone();
                ntype = "mac";
            }
            if src.is_some() && dst.is_some() {
                let src = src.unwrap();
                let dst = dst.unwrap();
                let mut fd: Option<FlowDirection> = None;
                let key = src.clone() + "=>" + &dst;
                match fm.get_mut(&key) {
                    None => {
                        fd = Some(FlowDirection {
                            agent: dg.agent.clone(),
                            utc: dg.unixSecondsUTC,
                            source: src, 
                            target: dst, 
                            srcport: srcport.unwrap_or(-1),
                            dstport: dstport.unwrap_or(-1),
                            ntype: ntype.to_string(), 
                            size: size * s.meanSkipCount
                        });
                    },
                    Some(ref mut s) => {
                        s.size += size;
                    }
                }
                if fd.is_some() {
                    let fd = fd.unwrap();
                    fm.insert(key, fd);
                }
            }
            
        }
    }
    Ok(fm)
}

#[allow(non_snake_case)]
pub fn read_sample_v5(s: &mut SampleV5) -> Result<(), Box<std::error::Error>> {
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("failed to read from pipe");
        //print!("{}", input);
        if let Some(_) = input.find("srcMAC ") {
            let srcMAC:String;
            try_scan!(input.bytes() => "srcMAC {}", srcMAC);
            s.srcMAC = Some(srcMAC);
        } else if let Some(_) = input.find("dstMAC ") {
            let dstMAC:String;
            try_scan!(input.bytes() => "dstMAC {}", dstMAC);
            s.dstMAC = Some(dstMAC);
        } else if let Some(_) = input.find("srcIP ") {
            let srcIP:String;
            try_scan!(input.bytes() => "srcIP {}", srcIP);
            s.srcIP = Some(srcIP);
        } else if let Some(_) = input.find("dstIP ") {
            let dstIP:String;
            try_scan!(input.bytes() => "dstIP {}", dstIP);
            s.dstIP = Some(dstIP);
        } else if let Some(_) = input.find("srcIP6 ") {
            let srcIP6:String;
            try_scan!(input.bytes() => "srcIP6 {}", srcIP6);
            s.srcIP6 = Some(srcIP6);
        } else if let Some(_) = input.find("dstIP6 ") {
            let dstIP6:String;
            try_scan!(input.bytes() => "dstIP6 {}", dstIP6);
            s.dstIP6 = Some(dstIP6);
        } else if let Some(_) = input.find("UDPSrcPort ") {
            let UDPSrcPort:i32;
            try_scan!(input.bytes() => "UDPSrcPort {}", UDPSrcPort);
            s.UDPSrcPort = Some(UDPSrcPort);
        } else if let Some(_) = input.find("UDPDstPort ") {
            let UDPDstPort:i32;
            try_scan!(input.bytes() => "UDPDstPort {}", UDPDstPort);
            s.UDPDstPort = Some(UDPDstPort);
        } else if let Some(_) = input.find("TCPSrcPort ") {
            let TCPSrcPort:i32;
            try_scan!(input.bytes() => "TCPSrcPort {}", TCPSrcPort);
            s.TCPSrcPort = Some(TCPSrcPort);
        } else if let Some(_) = input.find("TCPDstPort ") {
            let TCPDstPort:i32;
            try_scan!(input.bytes() => "TCPDstPort {}", TCPDstPort);
            s.TCPDstPort = Some(TCPDstPort);
        } else if let Some(_) = input.find("meanSkipCount ") {
            try_scan!(input.bytes() => "meanSkipCount {}", s.meanSkipCount);
        } else if let Some(_) = input.find("inputPort ") {
            try_scan!(input.bytes() => "inputPort {}", s.inputPort);
        } else if let Some(_) = input.find("outputPort ") {
            try_scan!(input.bytes() => "outputPort {}", s.outputPort);
        } else if let Some(_) = input.find("outputPort ") {
            try_scan!(input.bytes() => "sampleSequenceNo {}", s.sampleSequenceNo);
        } else if let Some(_) = input.find("sampledPacketSize ") {
            try_scan!(input.bytes() => "sampledPacketSize {}", s.sampledPacketSize);
        } else if let Some(_) = input.find("endSample") {
            break;
        }
    }
    Ok(())
}

pub fn read_datagram(dg: &mut Datagram) -> Result<(), Box<std::error::Error>> {
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("failed to read from pipe");
        
        if let Some(_) = input.find("agent ") {
            try_scan!(input.bytes() => "agent {}", dg.agent);
        } else if let Some(_) = input.find("datagramSourceIP ") {
            try_scan!(input.bytes() => "datagramSourceIP {}", dg.datagramSourceIP);
        } else if let Some(_) = input.find("unixSecondsUTC ") {
            try_scan!(input.bytes() => "unixSecondsUTC {}", dg.unixSecondsUTC);
        } else if let Some(_) = input.find("datagramVersion ") {
            try_scan!(input.bytes() => "datagramVersion {}", dg.datagramVersion);
        } else if let Some(_) = input.find("sysUpTime ") {
            try_scan!(input.bytes() => "sysUpTime {}", dg.sysUpTime);
        } else if let Some(_) = input.find("startSample") {
            let mut s:SampleV5 = Default::default();
            read_sample_v5(&mut s)?;
            dg.samplev5.push(s);
        } else if let Some(_) = input.find("endDatagram") {
            break;
        }
    }
    Ok(())
}

pub fn get_sql_url() -> Result<String, Box<std::error::Error>> {
    use std::env;
    let _ = dotenv::dotenv();
    let url = env::var("PSQL_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run unit tests");
    Ok(url)
}

pub fn connect_sql() -> Result<Connection, Box<std::error::Error>> {
    let url = get_sql_url()?;
    let conn = Connection::connect(url, TlsMode::None)?;
    Ok(conn)
}

pub fn input_data(conn: &Connection) -> Result<(), Box<std::error::Error>> {
    let mut data: Vec<Datagram> = vec![];
    let mut input = String::new();
    loop {
        io::stdin().read_line(&mut input).expect("failed to read from pipe");
        if let Some(_) = input.find("startDatagram") {
            let mut dg:Datagram = Default::default();
            read_datagram(&mut dg)?;
            data.push(dg);
            let mut fm = build_graph(&data)?;
            filter_multicast_ipv6(&mut fm);
            filter_local_ipv6(&mut fm);
            filter_ff_mac(&mut fm);
            if fm.len() > 0 {
                let mut multi_data = String::from("INSERT INTO flow (agent, utc, src, dst, srcport, dstport, ntype, size) VALUES \n");
                for (_k,v) in fm.iter() {
                    multi_data.push_str(&format!("('{}',{},'{}','{}',{},{},'{}',{}),", 
                        v.agent, v.utc, v.source, v.target, v.srcport, v.dstport, v.ntype, v.size));
                }
                multi_data.pop();
                match conn.execute(multi_data.as_str(), &[]) {
                    Ok(_)=>(),
                    Err(x)=> {
                        println!("SQL ERROR:\n{} \n{}", multi_data, x);
                        ()
                    }
                }
                //let (nodes_data, links_data) = build_d3_json(&fm)?;
                //println!("{}\n{}", nodes_data, links_data);
            }
        };
        data.clear();
    }
    //Ok(())
}
