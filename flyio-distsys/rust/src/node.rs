use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Display,
    time::SystemTime,
};

use rand::SeedableRng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeId(Option<String>, Option<u16>);

impl NodeId {
    fn new() -> Self {
        Self(None, None)
    }

    fn init(&mut self, id: String) {
        assert!(id.starts_with('n'));
        let num = id[1..].parse::<u16>().expect("ID should be of format 'n<number>'");
        self.0 = Some(id);
        self.1 = Some(num);
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => f.write_str(s),
            None => f.write_str("N/A"),
        }
    }
}

pub struct Node {
    id: NodeId,
    next_msg_id: u64,
    unique_id_generator: ulid::Generator,
    prng: rand::rngs::SmallRng,
    messages: BTreeSet<u64>,
    messages_by_node: HashMap<String, BTreeSet<u64>>,
    messages_pending: HashMap<u64, (SystemTime, u64, String)>,
    messages_pending_by_value: HashSet<(u64, String)>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            id: NodeId::new(),
            next_msg_id: 0,
            unique_id_generator: ulid::Generator::new(),
            prng: rand::rngs::SmallRng::from_entropy(),
            messages: BTreeSet::new(),
            messages_by_node: HashMap::new(),
            // Consider "bidi" map
            messages_pending: HashMap::new(),
            messages_pending_by_value: HashSet::new(),
        }
    }

    pub fn init(&mut self, id: String) {
        self.id.init(id);
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn id_str(&self) -> &str {
        self.id.0.as_ref().unwrap()
    }

    #[inline]
    pub fn get_next_msg_id(&mut self) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }

    #[inline]
    pub fn generate_unique_id(&mut self) -> ulid::Ulid {
        let now = SystemTime::now();
        self.unique_id_generator
            .generate_from_datetime_with_source(now, &mut self.prng)
            .expect("Random bits should not overflow - TODO test")
    }

    #[inline]
    pub fn add_message(&mut self, message: u64, from: &str) -> bool {
        if from.starts_with('n') {
            self.messages_by_node
                .raw_entry_mut()
                .from_key(from)
                .and_modify(|_, set| _ = set.insert(message))
                .or_insert_with(|| {
                    let mut set = BTreeSet::new();
                    set.insert(message);
                    (from.to_string(), set)
                });
        }
        self.messages.insert(message)
    }

    #[inline]
    pub fn node_acked_message(&mut self, message: u64, node: &str) {
        assert!(node.starts_with('n'));
        self.messages_by_node
            .raw_entry_mut()
            .from_key(node)
            .and_modify(|_, set| _ = set.insert(message))
            .or_insert_with(|| {
                let mut set = BTreeSet::new();
                set.insert(message);
                (node.to_string(), set)
            });
    }

    #[inline]
    pub fn get_messages(&self) -> Vec<u64> {
        self.messages.iter().cloned().collect()
    }

    #[inline]
    pub fn get_gossip_messages_for(&self, node: &str) -> Vec<u64> {
        let node_messages = self.messages_by_node.get(node);
        match node_messages {
            Some(set) => self.messages.difference(set).copied().collect(),
            None => self.messages.iter().copied().collect(),
        }
    }

    #[inline]
    pub fn add_pending_message(&mut self, msg_id: u64, message: u64, timestamp: SystemTime, node: &str) {
        assert!(self
            .messages_pending
            .insert(msg_id, (timestamp, message, node.to_string()))
            .is_none());

        self.messages_pending_by_value.insert((message, node.to_string()));
    }

    #[inline]
    pub fn message_is_pending(&self, message: u64, node: &str) -> bool {
        self.messages_pending_by_value.contains(&(message, node.to_string()))
    }

    #[inline]
    pub fn try_take_pending_message(&mut self, msg_id: u64) -> Option<(SystemTime, u64, String)> {
        let msg = self.messages_pending.remove(&msg_id);
        if let Some((_, message, node)) = &msg {
            assert!(self.messages_pending_by_value.remove(&(*message, node.clone())));
        }
        msg
    }

    #[inline]
    pub fn get_retryable_messages(&self, now: SystemTime) -> Vec<(SystemTime, u64, String)> {
        self.messages_pending
            .iter()
            .filter(|&(_, (timestamp, _, _))| {
                let duration = now
                    .duration_since(*timestamp)
                    .expect("clock should always move forwards");
                duration.as_secs_f64() > 0.1
            })
            .map(|(_, v)| v)
            .cloned()
            .collect()
    }
}

pub struct Topology {
    id: NodeId,
    topology: HashMap<String, Vec<String>>,
    neighbors: Vec<String>,
}

impl Topology {
    pub fn new() -> Self {
        Self {
            id: NodeId::new(),
            topology: HashMap::new(),
            neighbors: Vec::new(),
        }
    }

    pub fn init(&mut self, id: String) {
        self.id.init(id);
    }

    pub fn init_topology(&mut self, topology: HashMap<String, Vec<String>>) {
        self.neighbors = match topology.get(&self.id.0.clone().unwrap()) {
            Some(n) => n.clone(),
            None => Vec::new(),
        };
        self.topology = topology;
    }

    #[inline]
    pub fn get_my_neighbors(&self) -> &[String] {
        &self.neighbors
    }

    #[inline]
    pub fn get_neighbors(&self, node_id: &str) -> &[String] {
        match self.topology.get(node_id) {
            Some(neighbors) => neighbors,
            None => &[],
        }
    }
}
