use std::{collections::HashMap, fmt::Display, time::SystemTime};

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
    topology: HashMap<String, Vec<String>>,
    messages: rustc_hash::FxHashSet<i64>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            id: NodeId::new(),
            next_msg_id: 0,
            unique_id_generator: ulid::Generator::new(),
            prng: rand::rngs::SmallRng::from_entropy(),
            topology: HashMap::with_capacity(4),
            messages: rustc_hash::FxHashSet::default(),
        }
    }

    pub fn init(&mut self, id: String) {
        self.id.init(id);
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn set_topology(&mut self, topology: HashMap<String, Vec<String>>) {
        self.topology = topology;
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
    pub fn add_message(&mut self, message: i64) {
        self.messages.insert(message);
    }

    #[inline]
    pub fn get_messages(&self) -> Vec<i64> {
        self.messages.iter().cloned().collect()
    }
}
