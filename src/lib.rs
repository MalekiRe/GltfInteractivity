use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use serde::{Deserialize, Serialize};

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

pub trait NodeArchetypeBuilder: NodeBehavior {
    fn build(node_id: NodeId) -> NodeArchetypeIncomplete;
    fn new_node(node_id: NodeId) -> Box<dyn NodeBehavior>;
    fn name() -> String;
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeArchetypeIncomplete {
    pub node_id: NodeId,
    pub name: String,
    pub input_value_sockets: Vec<InputValueSocketIncomplete>,
    pub input_flow_sockets: Vec<InputFlowSocketIncomplete>,
    pub output_value_sockets: Vec<OutputValueSocketIncomplete>,
    pub output_flow_sockets: Vec<OutputFlowSocketIncomplete>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValueSocketIncomplete {
    name: String,
    pub output_value_socket: Option<crate::OutputValueSocketIncomplete>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValueSocketIncomplete {
    name: String,
    node_id: NodeId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFlowSocketIncomplete {
    name: String,
    node_id: NodeId,
    pub output_flow_socket: Option<crate::OutputFlowSocketIncomplete>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFlowSocketIncomplete {
    name: String,
    pub input_flow_socket: Option<Box<crate::InputFlowSocketIncomplete>>,
}

impl From<NodeArchetypeIncomplete> for NodeArchetype {
    fn from(value: NodeArchetypeIncomplete) -> Self {
        NodeArchetype {
            node_id: value.node_id,
            name: value.name,
            input_value_sockets: value.input_value_sockets.into_iter().map(|a| a.into()).collect(),
            input_flow_sockets: value.input_flow_sockets.into_iter().map(|a| a.into()).collect(),
            output_value_sockets: value.output_value_sockets.into_iter().map(|a| a.into()).collect(),
            output_flow_sockets: value.output_flow_sockets.into_iter().map(|a| a.into()).collect(),
        }
    }
}

impl From<OutputFlowSocketIncomplete> for OutputFlowSocket {
    fn from(value: OutputFlowSocketIncomplete) -> OutputFlowSocket {
        OutputFlowSocket {
            name: value.name,
            input_flow_socket: match value.input_flow_socket {
                None => None,
                Some(some) => {
                    Some(Box::new((*some).into()))
                }
            },
        }
    }
}

impl From<InputFlowSocketIncomplete> for InputFlowSocket {
    fn from(value: InputFlowSocketIncomplete) -> Self {
        InputFlowSocket {
            name: value.name,
            node_id: value.node_id,
            output_flow_socket: value.output_flow_socket.map(|a| a.into()),
        }
    }
}


impl From<InputValueSocketIncomplete> for InputValueSocket {
    fn from(value: InputValueSocketIncomplete) -> InputValueSocket {
        InputValueSocket {
            name: value.name,
            output_value_socket: value.output_value_socket.unwrap().into(),
        }
    }
}


impl From<OutputValueSocketIncomplete> for OutputValueSocket {
    fn from(value: OutputValueSocketIncomplete)-> OutputValueSocket {
        OutputValueSocket {
            name: value.name,
            node_id: value.node_id,
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeArchetype {
    pub node_id: NodeId,
    pub name: String,
    pub input_value_sockets: Vec<InputValueSocket>,
    pub input_flow_sockets: Vec<InputFlowSocket>,
    pub output_value_sockets: Vec<OutputValueSocket>,
    pub output_flow_sockets: Vec<OutputFlowSocket>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValueSocket {
    name: String,
    output_value_socket: OutputValueSocket,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValueSocket {
    name: String,
    node_id: NodeId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFlowSocket {
    name: String,
    node_id: NodeId,
    output_flow_socket: Option<OutputFlowSocket>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFlowSocket {
    name: String,
    input_flow_socket: Option<Box<InputFlowSocket>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeArchetypes(pub HashMap<NodeId, NodeArchetype>);

impl NodeArchetypes {
    pub fn add_archetype(&mut self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket], node: &dyn NodeBehavior) {
        let archetype = node.create_node_archetype(input_value_nodes, input_flow_nodes);
        self.0.insert(node.node_id(), archetype);
    }
}


pub struct ExistingValues(HashMap<NodeId, HashMap<String, Value>>);

impl Default for ExistingValues {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

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

pub struct NodeBehaviors(pub HashMap<NodeId, Box<dyn NodeBehavior>>);

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
#[derive(Clone, Serialize, Deserialize)]
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

impl NodeArchetypeBuilder for MathPi {
    fn build(node_id: NodeId) -> NodeArchetypeIncomplete {
        NodeArchetypeIncomplete {
            node_id,
            name: Self::name(),
            input_value_sockets: vec![],
            input_flow_sockets: vec![],
            output_value_sockets: vec![
                OutputValueSocketIncomplete {
                    name: "value".to_string(),
                    node_id,
                }
            ],
            output_flow_sockets: vec![],
        }
    }

    fn new_node(node_id: NodeId) -> Box<dyn NodeBehavior> {
        Box::new(Self::new(node_id))
    }

    fn name() -> String {
        "math/pi".to_string()
    }
}

impl NodeBehavior for MathPi {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, _: &[OutputValueSocket], _: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            name: "math/pi".to_string(),
            input_value_sockets: vec![],
            input_flow_sockets: vec![],
            output_value_sockets: vec![i
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
#[derive(Clone, Serialize, Deserialize)]
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

impl NodeArchetypeBuilder for MathAdd {
    fn build(node_id: NodeId) -> NodeArchetypeIncomplete {
        NodeArchetypeIncomplete {
            node_id,
            name: Self::name(),
            input_value_sockets: vec![
                InputValueSocketIncomplete {
                    name: "a".to_string(),
                    output_value_socket: None,
                },
                InputValueSocketIncomplete {
                    name: "b".to_string(),
                    output_value_socket: None,
                }
            ],
            input_flow_sockets: vec![],
            output_value_sockets: vec![
                OutputValueSocketIncomplete {
                    name: "value".to_string(),
                    node_id,
                }
            ],
            output_flow_sockets: vec![],
        }
    }

    fn new_node(node_id: NodeId) -> Box<dyn NodeBehavior> {
        Box::new(Self::new(node_id))
    }

    fn name() -> String {
        "math/add".to_string()
    }
}

impl NodeBehavior for MathAdd {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], _: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            name: "math/add".to_string(),
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

#[derive(Clone, Serialize, Deserialize)]
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

impl NodeArchetypeBuilder for PrintNode {
    fn build(node_id: NodeId) -> NodeArchetypeIncomplete {
        NodeArchetypeIncomplete {
            node_id,
            name: Self::name(),
            input_value_sockets: vec![
                InputValueSocketIncomplete {
                    name: "print_value".to_string(),
                    output_value_socket: None,
                }
            ],
            input_flow_sockets: vec![
                InputFlowSocketIncomplete {
                    name: "print_input".to_string(),
                    node_id,
                    output_flow_socket: None,
                }
            ],
            output_value_sockets: vec![],
            output_flow_sockets: vec![],
        }
    }

    fn new_node(node_id: NodeId) -> Box<dyn NodeBehavior> {
        Box::new(Self::new(node_id))
    }

    fn name() -> String {
        "custom/print".to_string()
    }
}

impl NodeBehavior for PrintNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            name: "custom/print".to_string(),
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

#[derive(Clone, Serialize, Deserialize)]
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

impl NodeArchetypeBuilder for SequenceNode {
    fn build(node_id: NodeId) -> NodeArchetypeIncomplete {
        NodeArchetypeIncomplete {
            node_id,
            name: Self::name(),
            input_value_sockets: vec![],
            input_flow_sockets: vec![
                InputFlowSocketIncomplete {
                    name: "in".to_string(),
                    node_id,
                    output_flow_socket: None,
                }
            ],
            output_value_sockets: vec![],
            output_flow_sockets: vec![
                OutputFlowSocketIncomplete {
                    name: "out".to_string(),
                    input_flow_socket: None,
                }
            ],
        }
    }

    fn new_node(node_id: NodeId) -> Box<dyn NodeBehavior> {
        Box::new(Self::new(node_id))
    }

    fn name() -> String {
        "flow/sequence".to_string()
    }
}

impl NodeBehavior for SequenceNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn create_node_archetype(&self, input_value_nodes: &[OutputValueSocket], input_flow_nodes: &[OutputFlowSocket]) -> NodeArchetype {
        NodeArchetype {
            node_id: self.node_id,
            name: "flow/sequence".to_string(),
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

static REGISTRY: OnceLock<Arc<Mutex<HashMap<String, (Box<fn(NodeId) -> NodeArchetypeIncomplete>, Box<fn(NodeId) -> Box<dyn NodeBehavior>>)>>>> = OnceLock::new();

pub fn get_registry() -> Arc<Mutex<HashMap<String, (Box<fn(NodeId) -> NodeArchetypeIncomplete>, Box<fn(NodeId) -> Box<dyn NodeBehavior>>)>>> {
    REGISTRY.get_or_init(|| {
        Arc::new(Mutex::new(HashMap::default()))
    }).clone()
}

pub trait RegisterNode {
    fn register();
}

impl<T: NodeArchetypeBuilder + NodeBehavior> RegisterNode for T {
    fn register() {
        get_registry()
            .lock().unwrap()
            .insert(T::name(), (Box::new(|node_id| {
                T::build(node_id)
            }), Box::new(|node_id| {
                T::new_node(node_id)
            })));
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