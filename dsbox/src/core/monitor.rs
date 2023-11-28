use regex::Regex;

use libproto::Message;

pub struct MonitorSession {
    source: String,
    src_match: Regex,
    dst_match: Regex,
}

impl MonitorSession {
    pub fn new(source: String, src_match: &str, dst_match: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            source,
            src_match: Regex::new(src_match)?,
            dst_match: Regex::new(dst_match)?,
        })
    }

    pub fn matches(&self, message: &Message) -> bool {
        self.src_match.is_match(&message.src) && self.dst_match.is_match(&message.dst)
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}