//! Monitor sessions are like "wiretaps" that a client may use to inspect network traffic
//!
//! A client can send a [`BeginMonitor`](libproto::system::BeginMonitor) [`Message`] to the core to start a new session.
//! Each monitor session filters all [`Message`] by their source and destination node names and if they match a given regex, the
//! node that started the session is notified upon sending and delivering of [`Message`] (via a [`MonitorEvent`](libproto::system::MonitorEvent) message).

use regex::Regex;

use libproto::Message;

/// A monitor session
pub struct MonitorSession {
    /// The name of the node that started the session
    source: String,
    /// The regex is matched against each [`Message`]s source node name
    src_match: Regex,
    /// The regex is matched against each [`Message`]s destination node name
    dst_match: Regex,
}

impl MonitorSession {
    /// creates a new [`MonitorSession`] with the given source name and regexes.
    pub fn new(source: String, src_match: &str, dst_match: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            source,
            src_match: Regex::new(src_match)?,
            dst_match: Regex::new(dst_match)?,
        })
    }

    /// Returns `true` if the given [`Message`] matches this session (i.e. the source node should be notified).
    pub fn matches(&self, message: &Message) -> bool {
        self.src_match.is_match(&message.src) && self.dst_match.is_match(&message.dst)
    }

    /// Returns the name of the node that started this session.
    pub fn source(&self) -> &str {
        &self.source
    }
}