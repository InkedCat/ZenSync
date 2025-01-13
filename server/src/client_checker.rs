use std::collections::HashMap;

use crate::config::Peer;

pub struct PeerChecker {
    peers: HashMap<String, Peer>,
}

impl PeerChecker {
    pub fn new(peers: HashMap<String, Peer>) -> Self {
        Self { peers }
    }

    pub fn get_peer(&self, public_key: &String) -> Option<&Peer> {
        self.peers.get(public_key)
    }

    pub fn has_client(&self, public_key: &String) -> bool {
        self.peers.contains_key(public_key)
    }
}
