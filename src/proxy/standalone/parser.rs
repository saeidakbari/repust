use crate::com::AsError;

pub struct ServerLine {
    addr: String,
    weight: usize,
    alias: Option<String>,
}

impl ServerLine {
    // parse_servers parse the server line into a list of ServerLine
    // for example: 192.168.1.2:1074:10 redis-20
    // the first part is the address (including port), the second part is the weight
    pub fn parse_servers(servers: &[String]) -> Result<Vec<ServerLine>, AsError> {
        let mut sl = Vec::with_capacity(servers.len());
        for server in servers {
            let mut iter = server.split(' ');
            let first_part = iter.next().expect("first partation must exists");
            if first_part.chars().filter(|x| *x == ':').count() == 1 {
                let alias = iter.next().map(|x| x.to_string());
                sl.push(ServerLine {
                    addr: first_part.to_string(),
                    weight: 1,
                    alias,
                });
            }

            let mut fp_sp = first_part.rsplitn(2, ':').filter(|x| !x.is_empty());
            let weight = {
                let weight_str = fp_sp.next().unwrap_or("1");
                weight_str.parse::<usize>()?
            };
            let addr = fp_sp.next().expect("addr never be absent").to_owned();
            drop(fp_sp);
            let alias = iter.next().map(|x| x.to_string());
            sl.push(ServerLine {
                addr,
                weight,
                alias,
            });
        }
        Ok(sl)
    }

    // split_spots splits the ServerLine into three parts: nodes, alias, weights
    pub fn split_spots(sls: &[ServerLine]) -> (Vec<String>, Vec<String>, Vec<usize>) {
        let mut nodes = Vec::new();
        let mut alias = Vec::new();
        let mut weights = Vec::new();

        for sl in sls {
            if sl.alias.is_some() {
                alias.push(
                    sl.alias
                        .as_ref()
                        .cloned()
                        .expect("node addr can't be empty"),
                );
            }
            nodes.push(sl.addr.clone());
            weights.push(sl.weight);
        }
        (nodes, alias, weights)
    }
}
