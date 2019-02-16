extern crate regex;

use super::toml;
use super::node;
use super::tree;
use super::toml::ValueImpl;

use std::str;
use std::str::FromStr;
use std::string::ToString;
use std::fs::File;
use std::io::prelude::*;
use self::regex::Regex;
use nom::IResult;

pub enum MatchString {
    Match(Regex),
    String(String),
}

pub enum CondType {
    Exists,
    Type(String),
    Equals(String),
    Matches(Vec<MatchString>),
    Smaller(String),
    Greater(String)
}

pub struct Cond {
    pub entry: String,
    pub cond_type: CondType,
}

pub enum CondNodeType {
    Not, // 1 child
    And, // n children
    Or, // n children
    Cond(Cond),
}

type CondNode = tree::Node<CondNodeType>;

pub struct MetaNode<'a, 'b: 'a, 'c: 'a, 'd> {
    pub node: &'a node::Node<'b, 'c>,
    pub meta: &'d toml::Value
}

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

pub fn matches(a: &toml::Value, b: &Vec<MatchString>) -> bool {
    match a {
        &toml::Value::String(ref s) => {
            b.iter().map(|given| {
                match given {
                    &MatchString::Match(ref a) => a.is_match(&s),
                    &MatchString::String(ref a) => s.contains(a),
                }
            }).all(|a| a)
        }, &toml::Value::Array(ref array) => {
            'outer: for given in b {
                for val in array {
                    let val = match val {
                        &toml::Value::String(ref a) => a,
                        _ => return false,
                    };

                    match given {
                        &MatchString::Match(ref a) =>
                            if a.is_match(&val) { continue 'outer; },
                        &MatchString::String(ref a) =>
                            if val == a { continue 'outer; },
                    }
                }

                return false;
            }

            true
        }, _ => false
    }
}

pub fn check_cond(node: &MetaNode, cond: &Cond) -> bool {
    // special: content
    if cond.entry == "c" {
        // TODO: check if type is text
        let mut s = String::new();
        let res = File::open(node.node.node_path())
            .and_then(|mut f| f.read_to_string(&mut s));
        if let Err(e) = res {
            println!("Failed to read '{}': {}", node.node.id(), e);
            return false;
        }

        if let &CondType::Matches(ref patterns) = &cond.cond_type {
            for pattern in patterns {
                match pattern {
                    &MatchString::Match(ref a) =>
                        if !a.is_match(&s) { return false; },
                    &MatchString::String(ref a) =>
                        if !s.contains(a) { return false; },
                }
            }
            return true
        } else if let &CondType::Equals(ref v) = &cond.cond_type {
            return s == *v;
        }
    }

    // otherwise: meta
    let v = node.meta.find(&cond.entry);
    if v.is_none() {
        return false;
    }

    let v = v.unwrap();
    match &cond.cond_type {
        &CondType::Exists => true,
        &CondType::Equals(ref value) => equals(&v, &value),
        &CondType::Matches(ref value) => matches(&v, &value),
        _ => {
            // TODO: implement!
            println!("not implemented");
            false
        }
    }
}

pub fn node_matches(node: &MetaNode, cond: &CondNode) -> bool {
    match &cond.data {
        &CondNodeType::Not => {
            !node_matches(node, cond.children.first()
                .expect("Invalid CondNode: 'not' needs a child"))
        }, &CondNodeType::And => {
            for child in &cond.children {
                if !node_matches(node, &child) {
                    return false;
                }
            }

            true
        }, &CondNodeType::Or => {
            for child in &cond.children {
                if node_matches(node, &child) {
                    return true;
                }
            }

            false
        }, &CondNodeType::Cond(ref cond) => {
            check_cond(node, &cond)
        }
    }
}

pub fn print_cond(cond: &CondNode) {
    match &cond.data {
        &CondNodeType::Not => {
            print!("!(");
            print_cond(cond.children.first()
                .expect("Invalid CondNode: 'not' needs a child"));
            print!(")");
        }, &CondNodeType::And => {
            print!("(");
            let mut first = true;
            for child in &cond.children {
                if first {
                    first = false;
                } else {
                    print!(" && ");
                }
                print_cond(&child);
            }
            print!(")");
        }, &CondNodeType::Or => {
            print!("(");
            let mut first = true;
            for child in &cond.children {
                if first {
                    first = false;
                } else {
                    print!(" || ");
                }
                print_cond(&child);
            }
            print!(")");
        }, &CondNodeType::Cond(ref cond) => {
            match &cond.cond_type {
                &CondType::Exists =>
                    print!("exists({})", &cond.entry),
                &CondType::Equals(ref value) =>
                    print!("({} == {})", &cond.entry, value),
                &CondType::Matches(ref vals) => {
                    print!("({} matches [", &cond.entry);
                    let mut first = true;
                    for v in vals {
                        if first {
                            first = false;
                        } else {
                            print!(", ");
                        }

                        match v {
                            &MatchString::String(ref a) => print!("{}", a),
                            &MatchString::Match(ref a) =>
                                print!("<{}>", a.as_str()),
                        }
                    }

                    print!("])");
                } _ => print!("<not implemented>"),
            }
        }
    }
}

// parser
named!(identifier<&str>, map_res!(is_not!(":=<>;|()"), str::from_utf8));

named!(value_string_unesc, is_not!("|;,()"));
named!(value_string_esc,
    delimited!(tag!("\""), take_until!("\""), tag!("\"")));
named!(value_string<&str>, map_res!(
    alt_complete!(value_string_unesc | value_string_esc),
    str::from_utf8));

named!(value_pattern<&str>, map_res!(
    delimited!(tag!("<"), take_until!(">"), tag!(">")),
    str::from_utf8));
named!(value_string_or_pattern<MatchString>, alt_complete!(
    map!(map_res!(value_pattern, Regex::new), MatchString::Match) |
    map!(map!(value_string, ToString::to_string), MatchString::String)));

// TODO: date or number for greater/smaller
named!(cond_value<CondType>, switch!(
    opt!(alt_complete!(
        tag!(":") |
        tag!("=") |
        tag!(">") |
        tag!("<"))),
    Some(b":") => map!(
        separated_nonempty_list_complete!(
            tag!(","),
            value_string_or_pattern),
        CondType::Matches) |
    Some(b"=") => map!(
        map!(value_string, ToString::to_string),
        CondType::Equals)  |
    Some(b">") => map!(
        map!(value_string, ToString::to_string),
        CondType::Greater) |
    Some(b"<") => map!(
        map!(value_string, ToString::to_string),
        CondType::Smaller) /* |
    NOTE: enable this to allow exist statements
    None => value!(CondType::Exists)
    */
));
named!(expr<CondNode>, alt_complete!(
    delimited!(tag!("("), and, tag!(")")) |
    do_parse!(
        entry: identifier >>
        cond: cond_value >>
        (CondNode::new(CondNodeType::Cond(Cond {
            entry: entry.to_string(),
            cond_type: cond,
        })))
    )
));

named!(not<CondNode>, alt_complete!(map!(
        preceded!(tag!("!"), expr),
        |expr| CondNode {
            children: vec!(expr),
            data: CondNodeType::Not
        }) | expr));

named!(or<CondNode>, map!(
    separated_nonempty_list_complete!(tag!("|"), not),
    |mut children| {
        if children.len() == 1 {
            children.pop().unwrap()
        } else {
            CondNode {
                children,
                data: CondNodeType::Or,
            }
        }
    }
));

named!(and<CondNode>, map!(
    separated_nonempty_list_complete!(tag!(";"), or),
    |mut children| {
        if children.len() == 1 {
            children.pop().unwrap()
        } else {
            CondNode {
                children,
                data: CondNodeType::And,
            }
        }
    }
));

pub fn parse_condition(pattern: &str) -> Result<CondNode, String> {
    match and(pattern.as_bytes()) {
        IResult::Done(rest, value) => {
            if rest.len() > 0 {
                // TODO: performance
                let str = match str::from_utf8(rest) {
                    Ok(a) => a,
                    Err(_) => return Err("Invalid condition: non-utf8 \
                        input sequence".to_string()),
                };
                Err(format!("Unexpected character {}",
                    str.chars().next().unwrap()))
            } else {
                Ok(value)
            }
        }, IResult::Error(err) =>
            Err(format!("Parse Error: {}", err)),
        IResult::Incomplete(needed) =>
            Err(format!("Incomplete condition. Needed: {:?}", needed)),
    }
}
