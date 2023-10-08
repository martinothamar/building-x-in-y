use std::{
    collections::{hash_map::RawEntryMut, BTreeSet, HashMap},
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
    latest_by_node: HashMap<String, u64>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            id: NodeId::new(),
            next_msg_id: 0,
            unique_id_generator: ulid::Generator::new(),
            prng: rand::rngs::SmallRng::from_entropy(),
            messages: BTreeSet::new(),
            latest_by_node: HashMap::new(),
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
            self.latest_by_node
                .raw_entry_mut()
                .from_key(from)
                .and_modify(|_, v| *v = message.max(*v))
                .or_insert_with(|| (from.to_string(), message));
        }
        self.messages.insert(message)
    }

    #[inline]
    pub fn is_message_known_by(&mut self, message: u64, node: &str) -> bool {
        assert!(node.starts_with('n'));

        let entry = self.latest_by_node.get(node);
        match entry {
            Some(&v) => v >= message,
            None => false,
        }
    }

    #[inline]
    pub fn get_messages(&self) -> Vec<u64> {
        self.messages.iter().cloned().collect()
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
