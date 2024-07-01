use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeId(pub Option<String>, pub Option<u16>);

impl NodeId {
    pub fn new() -> Self {
        Self(None, None)
    }

    pub fn init(&mut self, id: String) {
        assert!(id.starts_with('n'));
        let num = id[1..].parse::<u16>().expect("ID should be of format 'n<number>'");
        self.0 = Some(id);
        self.1 = Some(num);
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
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

    #[inline]
    pub fn get_all_other_nodes(&self) -> Vec<String> {
        self.topology
            .keys()
            .filter(|&n| n != self.id.0.as_ref().unwrap())
            .cloned()
            .collect()
    }

    #[inline]
    pub fn node_count(&self) -> usize {
        self.topology.keys().count()
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}
