use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Value {
    Float(f32),
    Bool(bool),
    Int(i32),
}

#[derive(Copy, Clone, Debug)]
pub enum ValueType {
    Float,
    Bool,
    Int,
}

pub type NodeId = u32;

#[derive(Debug)]
pub struct NodeArchetype {
    node_id: NodeId,
    input_value_sockets: Vec<InputValueSocket>,
    input_flow_sockets: Vec<InputFlowSocket>,
    output_value_sockets: Vec<OutputValueSocket>,
    output_flow_sockets: Vec<OutputFlowSocket>,
}
#[derive(Debug, Clone)]
pub struct InputValueSocket {
    name: String,
    output_value_socket: OutputValueSocket,
}
#[derive(Debug, Clone)]
pub struct OutputValueSocket {
    name: String,
    node_id: NodeId,
}
#[derive(Debug, Clone)]
pub struct InputFlowSocket {
    name: String,
    node_id: NodeId,
    output_flow_socket: Option<OutputFlowSocket>,
}
#[derive(Debug, Clone)]
pub struct OutputFlowSocket {
    name: String,
    input_flow_socket: Option<Box<InputFlowSocket>>,
}

#[derive(Debug)]
struct NodeArchetypes(HashMap<NodeId, NodeArchetype>);

impl NodeArchetypes {
    pub fn add_archetype(&mut self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket], node: &dyn NodeBehavior) {
        let archetype = node.create_node_archetype(input_value_nodes, input_flow_nodes);
        self.0.insert(node.node_id(), archetype);
    }
}


struct ExistingValues(HashMap<NodeId, HashMap<String, Value>>);

impl ExistingValues {
    pub fn run(&mut self, request: Vec<Request>, node_archetypes: &NodeArchetypes, node_behaviors: &mut NodeBehaviors) {
        for Request { node_id } in request {
            let behavior = node_behaviors.0.get(&node_id).unwrap();
            let requests = behavior.request(node_archetypes);
            drop(behavior);
            self.run(requests, node_archetypes, node_behaviors);
            let behavior = node_behaviors.0.get_mut(&node_id).unwrap();
            behavior.value(node_archetypes, self).unwrap();
        }
    }
    pub fn flow(&mut self, request: NodeId, node_archetypes: &NodeArchetypes, node_behaviors: &mut NodeBehaviors) {
        let behavior = node_behaviors.0.get(&request).unwrap();
        let requests = behavior.request(node_archetypes);
        drop(behavior);
        self.run(requests, node_archetypes, node_behaviors);
        let mut behavior = node_behaviors.0.remove(&request).unwrap();
        behavior.activate_input_node(node_archetypes, self, node_behaviors).unwrap();
        node_behaviors.0.insert(request, behavior);
    }
    pub fn set_value(&mut self, node_id: NodeId, name: impl ToString, value: Value) -> Option<()> {
        if self.0.get(&node_id).is_none() {
            self.0.insert(node_id, HashMap::new());
        }
        let temp = self.0.get_mut(&node_id).unwrap();
        temp.insert(name.to_string(), value);
        Some(())
    }
}

struct NodeBehaviors(HashMap<NodeId, Box<dyn NodeBehavior>>);

impl NodeBehaviors {
    pub fn add(&mut self, node: Box<dyn NodeBehavior>) {
        self.0.insert(node.node_id(), node);
    }
}

struct Request {
    node_id: NodeId,
}

struct Response(Value);

pub trait NodeBehavior {
    fn node_id(&self) -> NodeId;
    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket]) -> NodeArchetype;
    fn request(&self, node_archetypes: &NodeArchetypes) -> Vec<Request> {
        let mut requests = vec![];
        for input_value_socket in node_archetypes.0.get(&self.node_id()).unwrap().input_value_sockets.iter() {
            requests.push(Request {
                node_id: input_value_socket.output_value_socket.node_id,
            });
        }
        requests
    }
    fn value(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues) -> Option<()>;

    fn activate_input_node(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues, node_behaviors: &mut NodeBehaviors) -> Option<()>;
}

pub struct MathPi {
    node_id: NodeId
}
impl MathPi {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
        }
    }
}

impl NodeBehavior for MathPi {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, _: &[OutputValueSocket], _: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            input_value_sockets: vec![],
            input_flow_sockets: vec![],
            output_value_sockets: vec![
                OutputValueSocket {
                    name: "value".to_string(),
                    node_id: self.node_id,
                }
            ],
            output_flow_sockets: vec![],
        }
    }


    fn value(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues) -> Option<()> {
        existing_values.set_value(self.node_id, "value", Value::Float(std::f32::consts::PI));
        Some(())
    }

    fn activate_input_node(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues, _: &mut NodeBehaviors) -> Option<()> {
        todo!()
    }
}

pub struct MathAdd {
    node_id: NodeId
}

impl MathAdd {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
        }
    }
}

impl NodeBehavior for MathAdd {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], _: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            input_value_sockets: vec![
                InputValueSocket {
                    name: "a".to_string(),
                    output_value_socket: input_value_nodes.get(0).unwrap().clone(),
                },
                InputValueSocket {
                    name: "b".to_string(),
                    output_value_socket: input_value_nodes.get(1).unwrap().clone(),
                }
            ],
            input_flow_sockets: vec![],
            output_value_sockets: vec![
                OutputValueSocket {
                    name: "value".to_string(),
                    node_id: self.node_id,
                }
            ],
            output_flow_sockets: vec![],
        }
    }

    fn value(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues) -> Option<()> {
        let archetype = node_archetypes.0.get(&self.node_id())?;
        let a = archetype.input_value_sockets.get(0)?;
        let a = &a.output_value_socket;
        let val = existing_values.0.get(&a.node_id)?;
        let a = val.get(&a.name)?.clone();

        let b = archetype.input_value_sockets.get(0)?;
        let b = &b.output_value_socket;
        let val = existing_values.0.get(&b.node_id)?;
        let b = val.get(&b.name)?.clone();

        let a = match a {
            Value::Float(a) => a,
            _ => panic!(),
        };
        let b = match b {
            Value::Float(b) => b,
            _ => panic!(),
        };

        existing_values.set_value(self.node_id,"value", Value::Float(a + b))?;
        Some(())
    }

    fn activate_input_node(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues, _: &mut NodeBehaviors) -> Option<()> {
        todo!()
    }
}


pub struct PrintNode {
    node_id: NodeId,
}

impl PrintNode {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
        }
    }
}

impl NodeBehavior for PrintNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            input_value_sockets: vec![
                InputValueSocket {
                    name: "print_value".to_string(),
                    output_value_socket: input_value_nodes.first().unwrap().clone(),
                }
            ],
            input_flow_sockets: vec![
                InputFlowSocket {
                    name: "print_input".to_string(),
                    node_id: self.node_id,
                    output_flow_socket: input_flow_nodes.first().map(|a| a.clone()),
                }
            ],
            output_value_sockets: vec![],
            output_flow_sockets: vec![],
        }
    }

    fn value(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues) -> Option<()> {
        Some(())
    }

    fn activate_input_node(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues, _: &mut NodeBehaviors) -> Option<()> {
        let archetype = node_archetypes.0.get(&self.node_id)?;
        let a = archetype.input_value_sockets.get(0)?;
        let a = &a.output_value_socket;
        let val = existing_values.0.get(&a.node_id)?;
        let a = val.get(&a.name)?.clone();
        println!("{:#?}", a);
        Some(())
    }
}

pub struct SequenceNode {
    node_id: NodeId
}

impl SequenceNode {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
        }
    }
}

impl NodeBehavior for SequenceNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            input_value_sockets: vec![],
            input_flow_sockets: vec![
                InputFlowSocket {
                    name: "in".to_string(),
                    node_id: self.node_id,
                    output_flow_socket: input_flow_nodes.first().map(|a| a.clone()),
                }
            ],
            output_value_sockets: vec![],
            output_flow_sockets: input_flow_nodes.to_vec(),
        }
    }

    fn value(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues) -> Option<()> {
        panic!("cannot request value from this node");
    }

    fn activate_input_node(&mut self, node_archetypes: &NodeArchetypes, existing_values: &mut ExistingValues, node_behaviors: &mut NodeBehaviors) -> Option<()> {
        let node_archetype = node_archetypes.0.get(&self.node_id)?;
        for output_flow_socket in &node_archetype.output_flow_sockets {
            if let Some(input_flow_socket) = output_flow_socket.input_flow_socket.as_ref() {
                existing_values.flow(input_flow_socket.node_id, node_archetypes, node_behaviors);
            }
        }
        Some(())
    }
}

#[test]
fn test() {
    let mut existing_values = ExistingValues(Default::default());
    let mut archetypes = NodeArchetypes(HashMap::new());
    let mut node_behaviors = NodeBehaviors(HashMap::new());

    let pi_id = 0;
    let add_id = 1;
    let print_id = 2;


    let pi = MathPi::new(pi_id);
    let add = MathAdd::new(add_id);
    let print = PrintNode::new(print_id);
    archetypes.add_archetype(&[], &[], &pi);
    let pi_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: pi_id,
    };
    let add_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: add_id,
    };
    archetypes.add_archetype(&[pi_output.clone(), pi_output.clone()], &[], &add);
    archetypes.add_archetype(&[add_output.clone()], &[], &print);

    node_behaviors.add(Box::new(pi));
    node_behaviors.add(Box::new(add));
    node_behaviors.add(Box::new(print));

    let request = Request {
        node_id: add_id,
    };
    existing_values.run(vec![request], &archetypes, &mut node_behaviors);
    panic!("value is: {:#?}", existing_values.0.get(&1).unwrap().get("value").unwrap());
}


#[test]
fn test2() {
    let mut existing_values = ExistingValues(Default::default());
    let mut archetypes = NodeArchetypes(HashMap::new());
    let mut node_behaviors = NodeBehaviors(HashMap::new());

    let pi_id = 0;
    let add_id = 1;
    let print_id = 2;


    let pi = MathPi::new(pi_id);
    let add = MathAdd::new(add_id);
    let print = PrintNode::new(print_id);
    archetypes.add_archetype(&[], &[], &pi);
    let pi_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: pi_id,
    };
    let add_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: add_id,
    };
    archetypes.add_archetype(&[pi_output.clone(), pi_output.clone()], &[], &add);
    archetypes.add_archetype(&[add_output.clone()], &[], &print);

    node_behaviors.add(Box::new(pi));
    node_behaviors.add(Box::new(add));
    node_behaviors.add(Box::new(print));

    existing_values.flow(print_id, &archetypes, &mut node_behaviors);
}

#[test]
fn test3() {
    let mut existing_values = ExistingValues(Default::default());
    let mut archetypes = NodeArchetypes(HashMap::new());
    let mut node_behaviors = NodeBehaviors(HashMap::new());

    let pi_id = 0;
    let add_id = 1;
    let print_id = 2;
    let print2_id = 3;
    let seq_id = 4;


    let pi = MathPi::new(pi_id);
    let add = MathAdd::new(add_id);
    let print = PrintNode::new(print_id);
    let print2 = PrintNode::new(print2_id);
    let seq = SequenceNode::new(seq_id);

    let pi_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: pi_id,
    };
    let add_output = OutputValueSocket {
        name: "value".to_string(),
        node_id: add_id,
    };
    let seq_output1 = OutputFlowSocket {
        name: "first".to_string(),
        input_flow_socket: Some(Box::new(InputFlowSocket {
            name: "print_input".to_string(),
            node_id: print_id,
            output_flow_socket: None,
        })),
    };
    let seq_output2 = OutputFlowSocket {
        name: "second".to_string(),
        input_flow_socket: Some(Box::new(InputFlowSocket {
            name: "print_input".to_string(),
            node_id: print2_id,
            output_flow_socket: None,
        })),
    };
    archetypes.add_archetype(&[], &[], &pi);
    archetypes.add_archetype(&[pi_output.clone(), pi_output.clone()], &[], &add);
    archetypes.add_archetype(&[add_output.clone()], &[], &print);
    archetypes.add_archetype(&[pi_output.clone()], &[], &print2);

    archetypes.add_archetype(&[], &[seq_output1, seq_output2], &seq);

    node_behaviors.add(Box::new(pi));
    node_behaviors.add(Box::new(add));
    node_behaviors.add(Box::new(print));
    node_behaviors.add(Box::new(print2));
    node_behaviors.add(Box::new(seq));

    existing_values.flow(seq_id, &archetypes, &mut node_behaviors);
}