pub struct RSPEngine {
    pub query: String,
}

impl RSPEngine {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}
