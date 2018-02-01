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
    And, // n children
    Or, // n children
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
            let r = !node_matches_impl(&val, &pred, &pred.nodes[fc]);
            r
        }, &PredNode::And => {
            let mut it = root.first_child;
            while let Some(c) = it {
                let child = &pred.nodes[c];
                it = child.next_sibling;
                if !node_matches_impl(&val, &pred, &child) {
                    return false;
                }
            }

            true
        }, &PredNode::Or => {
            let mut it = root.first_child;
            while let Some(c) = it {
                let child = &pred.nodes[c];
                it = child.next_sibling;
                if node_matches_impl(&val, &pred, &child) {
                    return true;
                }
            }

            false
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


// parser
struct Pattern<'a> {
    s: &'a str
}

fn parse_vstring<'a>(pattern: &'a mut Pattern) -> Option<&'a str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[^\|;!\(\)=]+").unwrap();
    }

    let mut ret: Option<&'a str> = None;
    let mut end = 0;

    {
        let mat = match RE.find(&pattern.s) {
            Some(a) => a,
            _ => {
                println!("103");
                return None;
            },
        };

        if mat.start() != 0 {
            println!("104");
            return None;
        }

        ret = Some(mat.as_str());
        end = mat.end();
    }

    pattern.s = &pattern.s[end..];
    ret
}

fn parse_identifier<'a>(pattern: &'a mut Pattern) -> Option<&'a str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[^\|;!,\(\):=]+").unwrap();
    }

    let mut ret: Option<&'a str> = None;
    let mut end = 0;

    {
        let mat = match RE.find(&pattern.s) {
            Some(a) => a,
            _ => {
                println!("102");
                return None;
            },
        };

        if mat.start() != 0 {
            println!("101");
            return None;
        }

        ret = Some(mat.as_str());
        end = mat.end();
    }

    pattern.s = &pattern.s[end..];
    ret
}

fn parse_atom(pattern: &mut Pattern, pred: &mut NodePred, root: usize)
        -> Option<usize> {
    if let Some(n) = pattern.s.chars().next() {
        // parse parantheses expression
        if n == '(' {
            pattern.s = &pattern.s[1..];
            let a = parse_and(pattern, pred, root);
            if a.is_none() {
                println!("208");
                None
            } else if let Some(n) = pattern.s.chars().next() {
                if n == ')' {
                    pattern.s = &pattern.s[1..];
                    a
                } else {
                    println!("207");
                    None
                }
            } else {
                println!("206");
                None
            }
        // parse normal predicate
        } else {
            // entry
            let mut npred = Pred {
                pred_type: PredType::Exists, // dummy
                entry: match parse_identifier(pattern) {
                    Some(a) => a.to_string(),
                    _ => {
                        println!("205");
                        return None;
                    },
                }, 
            };

            if let Some(n) = pattern.s.chars().next() {
                pattern.s = &pattern.s[1..];
                let v = match parse_vstring(pattern) {
                    Some(a) => a,
                    _ => {
                        println!("204");
                        return None;
                    },
                };

                if n == '=' { // parse equal
                    npred.pred_type = PredType::Equals(v.to_string());
                } else if n == ':' { // parse match
                    npred.pred_type = PredType::Matches(v.to_string());
                } else {
                    println!("203: {}", n);
                    return None;
                }

                let id = pred.add(root, PredNode::Pred(npred));
                Some(id)
            } else {
                println!("202");
                None
            }
        }
    } else  {
        println!("201");
        None
    }
}

fn parse_not(pattern: &mut Pattern, pred: &mut NodePred, root: usize)
        -> Option<usize> {
    if let Some(n) = pattern.s.chars().next() {
        if n == '!' {
            let nroot = pred.add(root, PredNode::Not);
            pattern.s = &pattern.s[1..];
            parse_atom(pattern, pred, nroot)
        } else {
            parse_atom(pattern, pred, root)
        } 
    } else {
        println!("7");
        None
    }
}

fn parse_or(pattern: &mut Pattern, pred: &mut NodePred, root: usize) 
        -> Option<usize> {
    let oroot = pred.add(root, PredNode::Or);
    if parse_not(pattern, pred, oroot).is_none() {
        println!("6");
        return None;
    }

    while let Some(n) = pattern.s.chars().next() {
        if n == '|' {
            pattern.s = &pattern.s[1..];
            if parse_or(pattern, pred, oroot).is_none() {
                println!("4");
                return None;
            }
        } else {
            break;
        }
    }

    Some(oroot)
}

fn parse_and_impl(pattern: &mut Pattern, pred: &mut NodePred, node: usize) 
        -> Option<usize> {
    if parse_or(pattern, pred, node).is_none() {
        println!("3");
        return None;
    }

    while let Some(n) = pattern.s.chars().next() {
        if n == ';' {
            pattern.s = &pattern.s[1..];
            if parse_or(pattern, pred, node).is_none() {
                println!("2");
                return None;
            }
        } else {
            break;
        }
    }

    Some(node)
}

fn parse_and(pattern: &mut Pattern, pred: &mut NodePred, root: usize) 
        -> Option<usize> {
    let aroot = pred.add(root, PredNode::And);
    parse_and_impl(pattern, pred, aroot)
}

pub fn parse_pattern(s: &str) -> Option<NodePred> {
    let mut pattern = Pattern { s };
    let mut tree = NodePred::new();

    if pattern.s.len() == 0 {
        return Some(tree);
    }

    let root = tree.add_root(PredNode::And);
    if parse_and_impl(&mut pattern, &mut tree, root).is_none() {
        return None;
    }

    if pattern.s.len() > 0 {
        println!("Expected ';', got '{}'", pattern.s.chars().next().unwrap());
        return None;
    }

    Some(tree)
}

// debug function that prints out a preidcate tree
fn print_pred_impl(val: &toml::Value, pred: &NodePred, 
        root: &Node<PredNode>) {
    match &root.data {
        &PredNode::Not => {
            print!("!(");
            let fc = root.first_child.unwrap();
            print_pred_impl(&val, &pred, &pred.nodes[fc]);
            print!(")");
        }, &PredNode::And => {
            print!("(");
            let mut it = root.first_child;
            while let Some(c) = it {
                let child = &pred.nodes[c];
                print_pred_impl(&val, &pred, &child);
                it = child.next_sibling;
                if it.is_some() {
                    print!(" && ");
                }
            }

            print!(")");
        }, &PredNode::Or => {
            print!("(");
            let mut it = root.first_child;
            while let Some(c) = it {
                let child = &pred.nodes[c];
                print_pred_impl(&val, &pred, &child);
                it = child.next_sibling;
                if it.is_some() {
                    print!(" || ");
                }
            }

            print!(")");
        }, &PredNode::Pred(ref pred) => {
            match &pred.pred_type {
                &PredType::Exists => 
                    print!("exists({})", &pred.entry),
                &PredType::Equals(ref value) =>
                    print!("({} == {})", &pred.entry, value),
                &PredType::Matches(ref value) =>
                    print!("({} matches {})", &pred.entry, value),
                _ => print!("<not implemented>"),
            }
        }
    }
}

fn print_pred(val: &toml::Value, pred: &NodePred) {
    if let Some(r) = pred.root {
        print_pred_impl(val, pred, &pred.nodes[r]);
    }
}
