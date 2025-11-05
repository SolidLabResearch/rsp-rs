#[derive(Debug, Clone)]
pub enum Operator {
    RStream,
    IStream,
    DStream,
}

#[derive(Debug, Clone)]
pub struct R2S {
    pub operator: Operator,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct WindowDefinition {
    pub window_name: String,
    pub stream_name: String,
    pub width: i64,
    pub slide: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub sparql_query: String,
    pub r2s: R2S,
    pub s2r: Vec<WindowDefinition>,
}

impl ParsedQuery {
    pub fn new(sparql_query: String) -> Self {
        Self {
            sparql_query,
            r2s: R2S {
                operator: Operator::RStream,
                name: "undefined".to_string(),
            },
            s2r: Vec::new(),
        }
    }

    pub fn set_sparql_query(&mut self, query: String) {
        self.sparql_query = query;
    }

    pub fn set_r2s(&mut self, operator: Operator, name: String) {
        self.r2s = R2S { operator, name };
    }

    pub fn add_s2r_window(&mut self, window: WindowDefinition) {
        self.s2r.push(window);
    }
}
