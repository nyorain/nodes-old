extern crate toml;

use std::str::FromStr;
use parse;
use regex::Regex;

pub struct Node<T> {
    parent: Option<usize>,
    first_child: Option<usize>,
    last_child: Option<usize>,
    prev_sibling: Option<usize>,
    next_sibling: Option<usize>,
    data: T,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Node<T> {
        Node::<T> {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            data
        }
    }
}

#[derive(Default)]
pub struct Tree<T> {
    nodes: Vec<Node<T>>,
    root: Option<usize>
}

impl<T> Tree<T> {
    pub fn new() -> Tree<T> {
        Tree {
            nodes: Vec::new(),
            root: None
        }
    }

    pub fn get(&self, id: usize) -> &Node<T> {
        &self.nodes[id]
    }

    pub fn get_mut(&mut self, id: usize) -> &mut Node<T> {
        &mut self.nodes[id]
    }

    pub fn root(&self) -> Option<&Node<T>> {
        match self.root {
            Some(id) => Some(self.get(id)),
            None => None
        }
    }

    pub fn root_mut(&mut self) -> Option<&mut Node<T>> {
        match self.root {
            Some(id) => Some(self.get_mut(id)),
            None => None
        }
    }

    pub fn add_root(&mut self, data: T) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::new(data));

        {
            let ref mut last = self.nodes[id];
            last.first_child = self.root;
            last.last_child = self.root;
        }

        if let Some(old) = self.root {
            self.nodes[old].parent = Some(self.nodes.len());
        }

        self.root = Some(id);
        id
    }

    pub fn add(&mut self, parent: usize, data: T) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::new(data));
        if let Some(lc) = self.nodes[parent].last_child {
            self.nodes[parent].last_child = Some(id);
            self.nodes[lc].next_sibling = Some(id);
            self.nodes[id].prev_sibling = Some(lc);
        } else {
            let ref mut parent = self.nodes[parent];
            parent.first_child = Some(id);
            parent.last_child = Some(id);
        }
        id
    }
}

pub enum PredType {
    Exists,
    Type(String),
    Equals(String),
    Matches(String),
    Smaller(String),
    Greater(String),
}

pub struct Pred {
    pub entry: String,
    pub pred_type: PredType,
}

pub enum PredNode {
    Not, // 1 child
    And, // 2 childs
    Or, // 2 childs
    Pred(Pred), 
}

pub type NodePred = Tree<PredNode>;

pub fn equals(a: &toml::Value, b: &str) -> bool {
    match a {
        &toml::Value::String(ref s) => s == b,
        &toml::Value::Integer(ref i) => {
            if let Ok(b) = i64::from_str(b) { *i == b } else { false } 
        } &toml::Value::Float(ref f) => {
            if let Ok(b) = f64::from_str(b) { *f == b } else { false } 
        } &toml::Value::Array(ref a) => {
            let mut b = b.split(',');
            for val in a {
                let n = b.next();
                if n.is_none() || !equals(&val, n.unwrap()) {
                    return false;
                }
            }

            true
        }, _ => false
    }
}

pub fn matches(a: &toml::Value, b: &str) -> bool {
    match a {
        &toml::Value::String(ref s) => {
            let re = Regex::new(b);
            if re.is_err() {
                println!("Invalid regex: {}", re.unwrap_err());
                return false;
            }

            let re = re.unwrap();
            re.is_match(&s)
        }, &toml::Value::Array(ref a) => {
            'outer:
            for v1 in b.split(',') {

                // check if regex
                let mut re: Option<Regex> = None;
                if v1.bytes().next().unwrap() == b':' {
                    let rre = Regex::new(&v1[1..]);
                    if let Err(e) = rre {
                        println!("Invalid regex: {}", e);
                        return false;
                    }

                    re = rre.ok();
                }

                for v2 in a {
                    let v2 = v2.as_str();
                    if v2.is_none() {
                        return false;
                    }

                    let v2 = v2.unwrap();
                    if let Some(ref r) = re {
                        if r.is_match(v2) {
                            continue 'outer;
                        }
                    } else {
                        if v2 == v1 {
                            continue 'outer;
                        }
                    }
                }

                return false;
            }

            true
        }, _ => false
    }
}

pub fn check_predicate(val: &toml::Value, pred: &Pred) -> bool {
    let v = parse::toml_get(val, &pred.entry);
    if v.is_none() {
        return false;
    }

    let v = v.unwrap();
    match &pred.pred_type {
        &PredType::Exists => true,
        &PredType::Equals(ref value) => equals(&v, &value),
        &PredType::Matches(ref value) => matches(&v, &value),
        _ => {
            // TODO: implement!
            println!("not implemented");
            false
        }
    }
}

fn node_matches_impl(val: &toml::Value, pred: &NodePred, 
        root: &Node<PredNode>) -> bool {
    match &root.data {
        &PredNode::Not => {
            let fc = root.first_child.unwrap();
            !node_matches_impl(&val, &pred, &pred.nodes[fc])
        }, &PredNode::And => {
            let c1 = root.first_child.unwrap();
            let c2 = root.last_child.unwrap();
            node_matches_impl(&val, &pred, &pred.nodes[c1]) &&
                node_matches_impl(&val, &pred, &pred.nodes[c2])
        }, &PredNode::Or => {
            let c1 = root.first_child.unwrap();
            let c2 = root.last_child.unwrap();
            node_matches_impl(&val, &pred, &pred.nodes[c1]) &&
                node_matches_impl(&val, &pred, &pred.nodes[c2])
        }, &PredNode::Pred(ref pred) => {
            check_predicate(&val, &pred)
        }
    }
}

pub fn node_matches(val: &toml::Value, pred: &NodePred) -> bool {
    // always true for an empty predicate
    match pred.root() {
        Some(r) => node_matches_impl(&val, &pred, &r),
        None => true
    }
}

