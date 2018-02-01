pub struct Node<T> {
    pub children: Vec<Node<T>>,
    pub data: T,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Node<T> {
        Node {
            children: Vec::new(),
            data
        }
    }

    pub fn add_child(&mut self, data: T) -> usize {
        self.children.push(Node::new(data));
        self.children.len() - 1
    }
}
