use std::collections::HashMap;

pub struct Headers<'a>(HashMap<&'a str, &'a str>);

impl<'a> Headers<'a> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| *v)
    }

    pub fn new(map: HashMap<&'a str, &'a str>) -> Headers<'a> {
        Headers(map)
    }
}
