use crate::parsed_query::{Operator, ParsedQuery, WindowDefinition};
use regex::Regex;
use std::collections::HashMap;

pub struct RSPQLParser {
    pub rspql_query: String,
}

impl RSPQLParser {
    pub fn new(query: String) -> Self {
        Self { rspql_query: query }
    }

    pub fn parse(&self) -> ParsedQuery {
        let mut parsed = ParsedQuery::new("".to_string());
        let mut sparql_lines: Vec<String> = Vec::new();
        let mut prefix_mapper: HashMap<String, String> = HashMap::new();

        for line in self.rspql_query.lines() {
            let trimmed_line = line.trim();
            if trimmed_line.starts_with("REGISTER") {
                let re = Regex::new(r"REGISTER +([^ ]+) +<([^>]+)> AS").unwrap();
                for captures in re.captures_iter(trimmed_line) {
                    let op_str = captures.get(1).unwrap().as_str();
                    let name = captures.get(2).unwrap().as_str();
                    if let Some(operator) = Self::parse_operator(op_str) {
                        parsed.set_r2s(operator, name.to_string());
                    }
                }
            } else if trimmed_line.starts_with("FROM NAMED WINDOW") {
                let re = Regex::new(r"FROM +NAMED +WINDOW +([^ ]+) +ON +STREAM +([^ ]+) +\[RANGE +([^ ]+) +STEP +([^ ]+)\]").unwrap();
                for captures in re.captures_iter(trimmed_line) {
                    let window_name =
                        Self::unwrap(captures.get(1).unwrap().as_str(), &prefix_mapper);
                    let stream_name =
                        Self::unwrap(captures.get(2).unwrap().as_str(), &prefix_mapper);
                    let width = captures
                        .get(3)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .unwrap_or(0);
                    let slide = captures
                        .get(4)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .unwrap_or(0);
                    let window_def = WindowDefinition {
                        window_name,
                        stream_name,
                        width,
                        slide,
                    };
                    parsed.add_s2r_window(window_def);
                }
            } else {
                let mut sparql_line = trimmed_line.to_string();
                if sparql_line.starts_with("WINDOW") {
                    sparql_line = sparql_line.replace("WINDOW", "GRAPH");
                }
                if sparql_line.starts_with("PREFIX") {
                    let re = Regex::new(r"PREFIX +([^:]*): +<([^>]+)>").unwrap();
                    for captures in re.captures_iter(&sparql_line) {
                        let prefix = captures.get(1).unwrap().as_str().to_string();
                        let iri = captures.get(2).unwrap().as_str().to_string();
                        prefix_mapper.insert(prefix, iri);
                    }
                }
                sparql_lines.push(sparql_line);
            }
        }
        parsed.set_sparql_query(sparql_lines.join("\n"));
        parsed
    }

    fn parse_operator(op_str: &str) -> Option<Operator> {
        match op_str {
            "RStream" => Some(Operator::RStream),
            "IStream" => Some(Operator::IStream),
            "DStream" => Some(Operator::DStream),
            _ => None,
        }
    }

    fn unwrap(prefixed_iri: &str, mapper: &HashMap<String, String>) -> String {
        let trimmed = prefixed_iri.trim();
        if trimmed.starts_with('<') && trimmed.ends_with('>') {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() == 2 {
                if let Some(iri) = mapper.get(parts[0]) {
                    format!("{}{}", iri, parts[1])
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            }
        }
    }
}
