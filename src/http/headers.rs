use std::collections::HashMap;

pub struct Headers<'a>(HashMap<&'a str, &'a str>);

impl<'a> Headers<'a> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key.to_ascii_lowercase().as_str()).copied()
    }

    pub fn new(map: HashMap<&'a str, &'a str>) -> Headers<'a> {
        Headers(map)
    }
}
