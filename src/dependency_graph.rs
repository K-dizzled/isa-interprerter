use crate::instruction::{Instruction, LabeledInstruction, MemoryAccessMode, Reference};
use dot_writer::{Attributes, Color, DotWriter, Style};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Display;
use std::rc::Rc;
use rand::Rng;

type PropagateId = String;

#[derive(Debug, PartialEq, Clone)]
pub struct Propagate {
    pub associated_write: LabeledInstruction,
    pub to_location: Reference,
}

impl Propagate {
    pub fn new(associated_write: LabeledInstruction, to_location: Reference) -> Self {
        Self {
            associated_write,
            to_location,
        }
    }

    pub fn id(&self) -> PropagateId {
        format!("prop_{}", self.associated_write.id())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    Instruction(LabeledInstruction),
    Propagate(Propagate),
}

impl NodeType {
    pub fn id(&self) -> String {
        match self {
            Self::Instruction(instruction) => instruction.id(),
            Self::Propagate(propagate) => propagate.id(),
        }
    }

    pub fn thread_id(&self) -> usize {
        match self {
            Self::Instruction(instruction) => instruction.thread_id,
            Self::Propagate(propagate) => propagate.associated_write.thread_id,
        }
    }

    pub fn to_dot(&self) -> String {
        match self {
            Self::Instruction(instruction) => {
                format!("T{}Xinstr{}", instruction.thread_id.to_string(), instruction.line_index.to_string())
            }
            Self::Propagate(propagate) => format!(
                "T{}Xprop{}",
                propagate.associated_write.thread_id.to_string(),
                propagate.associated_write.line_index.to_string()
            ),
        }
    }
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Instruction(instruction) => write!(f, "{}", instruction.to_string()),
            NodeType::Propagate(propagate) => write!(
                f,
                "Propagate for write ({})",
                propagate.associated_write.to_string()
            ),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct InstructionNode {
    pub instruction: NodeType,
    pub depends_on: Vec<Rc<RefCell<InstructionNode>>>,
    pub depends_on_me: Vec<Rc<RefCell<InstructionNode>>>,
}

impl InstructionNode {
    fn new(instruction: LabeledInstruction) -> Rc<RefCell<InstructionNode>> {
        Rc::new(RefCell::new(Self {
            instruction: NodeType::Instruction(instruction),
            depends_on: Vec::new(),
            depends_on_me: Vec::new(),
        }))
    }

    fn new_propagate(
        write: LabeledInstruction,
        to_location: Reference,
    ) -> Rc<RefCell<InstructionNode>> {
        Rc::new(RefCell::new(Self {
            instruction: NodeType::Propagate(Propagate::new(write, to_location)),
            depends_on: Vec::new(),
            depends_on_me: Vec::new(),
        }))
    }

    pub fn add_dependency(from: Rc<RefCell<InstructionNode>>, to: Rc<RefCell<InstructionNode>>) {
        if from
            .borrow()
            .depends_on
            .clone()
            .into_iter()
            .any(|x| x.borrow().instruction.id() == to.borrow().instruction.id())
        {
            return;
        }
        // println!("Adding dependency from {} to {}", from.borrow().instruction.id(), to.borrow().instruction.id());
        from.borrow_mut().depends_on.push(to.clone());
        to.borrow_mut().depends_on_me.push(from.clone());
    }
}

pub struct DependencyGraph {
    pub nodes: Vec<Rc<RefCell<InstructionNode>>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_propagate(
        &mut self,
        write: LabeledInstruction,
        to_location: Reference,
    ) -> Rc<RefCell<InstructionNode>> {
        let node = InstructionNode::new_propagate(write, to_location);
        self.nodes.push(node.clone());
        node
    }

    pub fn add_node(&mut self, instruction: LabeledInstruction) -> Rc<RefCell<InstructionNode>> {
        let node = InstructionNode::new(instruction);
        self.nodes.push(node.clone());
        node
    }

    pub fn build_dependencies(&mut self) {
        let node_count = self.nodes.len();
        for index in 0..node_count {
            self.add_dependencies(index);
        }
    }

    pub fn dfs_filter_aux(
        &self,
        node: &Rc<RefCell<InstructionNode>>,
        visited: &mut HashSet<String>,
        result: &mut Vec<Rc<RefCell<InstructionNode>>>,
        predicate: &impl Fn(&NodeType) -> bool,
    ) {
        // Check id
        if visited.contains(&node.borrow().instruction.id()) {
            return;
        }
        visited.insert(node.borrow().instruction.id());
        if predicate(&node.borrow().instruction) {
            result.push(node.clone());
        }
        for dependency in &node.borrow().depends_on {
            self.dfs_filter_aux(dependency, visited, result, predicate);
        }
    }

    pub fn dfs_filter(
        &self,
        predicate: impl Fn(&NodeType) -> bool,
    ) -> Vec<Rc<RefCell<InstructionNode>>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut result = Vec::new();
        for node in &self.nodes {
            self.dfs_filter_aux(node, &mut visited, &mut result, &predicate);
        }
        result
    }

    fn add_rel_deps(&self, cur_node: &mut Rc<RefCell<InstructionNode>>) {
        let instr: NodeType = cur_node.borrow().instruction.clone();
        match instr {
            NodeType::Instruction(cur_instr) => {
                let depended_nodes = self.dfs_filter(|other_node| {
                    if let NodeType::Instruction(other_instr) = other_node {
                        cur_instr.thread_id == other_instr.thread_id
                            && cur_instr.line_index < other_instr.line_index
                    } else {
                        false
                    }
                });
                for depended_node in depended_nodes {
                    InstructionNode::add_dependency(depended_node.clone(), cur_node.clone());
                }
            }
            _ => {}
        }
    }

    fn add_acq_deps(&self, cur_node: &mut Rc<RefCell<InstructionNode>>) {
        let instr: NodeType = cur_node.borrow().instruction.clone();

        match instr {
            NodeType::Instruction(cur_instr) => {
                let dependant_nodes = self.dfs_filter(|other_node| {
                    if let NodeType::Instruction(other_instr) = other_node {
                        cur_instr.thread_id == other_instr.thread_id
                            && cur_instr.line_index > other_instr.line_index
                    } else {
                        false
                    }
                });
                for dependant_node in dependant_nodes {
                    InstructionNode::add_dependency(cur_node.clone(), dependant_node.clone());
                }
            }
            _ => {}
        }
    }

    pub fn add_dependencies(&mut self, node_index: usize) {
        let mut node = self.nodes[node_index].clone();
        fn get_access_mode_seq_cst(
            instruction: &Instruction,
            prev_am: MemoryAccessMode,
        ) -> MemoryAccessMode {
            match instruction {
                Instruction::Load(am, _, _) => {
                    if *am == MemoryAccessMode::SeqCst {
                        MemoryAccessMode::Acq
                    } else {
                        prev_am
                    }
                }
                Instruction::Store(am, _, _) => {
                    if *am == MemoryAccessMode::SeqCst {
                        MemoryAccessMode::Rel
                    } else {
                        prev_am
                    }
                }
                Instruction::Cas(_, am, _, _, _) | Instruction::Fai(_, am, _, _) => {
                    if *am == MemoryAccessMode::SeqCst {
                        MemoryAccessMode::RelAcq
                    } else {
                        prev_am
                    }
                }
                _ => MemoryAccessMode::Rlx,
            }
        }
        let c_node: NodeType = node.borrow().instruction.clone();
        match c_node {
            NodeType::Instruction(instruction) => match instruction.instruction {
                Instruction::Load(am, _, _)
                | Instruction::Store(am, _, _)
                | Instruction::Cas(_, am, _, _, _)
                | Instruction::Fai(_, am, _, _)
                | Instruction::Fence(am) => {
                    let modified_am = get_access_mode_seq_cst(&instruction.instruction, am);
                    match modified_am {
                        MemoryAccessMode::Rel => {
                            self.add_rel_deps(&mut node);
                        }
                        MemoryAccessMode::Acq => {
                            self.add_acq_deps(&mut node);
                        }
                        MemoryAccessMode::RelAcq => {
                            self.add_rel_deps(&mut node);
                            self.add_acq_deps(&mut node);
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn get_leaves(&self) -> Vec<Rc<RefCell<InstructionNode>>> {
        self.nodes
            .iter()
            .filter(|node| node.borrow().depends_on.is_empty())
            .cloned()
            .collect()
    }

    pub fn remove_node(
        &mut self,
        node: Rc<RefCell<InstructionNode>>,
        propagate: Option<(LabeledInstruction, Reference)>,
        pso: bool,
    ) {
        // Node has no outgoing edges
        if !node.borrow().depends_on.is_empty() {
            panic!("Cannot remove node with dependencies");
        }

        // Remove node incoming edges
        for dependency in &node.borrow().depends_on_me {
            dependency
                .borrow_mut()
                .depends_on
                .retain(|n| !Rc::ptr_eq(n, &node));
        }

        // Remove node from graph
        self.nodes.retain(|n| !Rc::ptr_eq(n, &node));

        if let Some((instr, to_loc)) = propagate {
            // println!("Propagating {:?}", instr);
            let propagate_node = self.add_propagate(instr.clone(), to_loc.clone());

            // Add dependencies from fences
            let dependant_nodes = self.dfs_filter(|other_node| {
                if let NodeType::Instruction(LabeledInstruction {
                    instruction: Instruction::Fence(_),
                    ..
                }) = other_node
                {
                    if let NodeType::Instruction(other_instr) = other_node {
                        other_instr.thread_id == instr.thread_id
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            for dependant_node in dependant_nodes {
                InstructionNode::add_dependency(dependant_node.clone(), propagate_node.clone());
            }

            // Add dependencies to other propagates
            let depended_nodes = self.dfs_filter(|other_node| {
                if let NodeType::Propagate(Propagate {
                    to_location: other_loc,
                    associated_write: labeled_instr,
                }) = other_node
                {
                    if pso {
                        (*other_loc) == to_loc && labeled_instr.thread_id == instr.thread_id && labeled_instr.line_index != instr.line_index
                    } else {
                        labeled_instr.thread_id == instr.thread_id && labeled_instr.line_index != instr.line_index
                    }
                } else {
                    false
                }
            });

            for depended_node in depended_nodes {
                InstructionNode::add_dependency(propagate_node.clone(), depended_node.clone());
            }
        }
    }

    pub fn to_dot(&self) -> String {
        fn get_color() -> Color {
            let mut rng = rand::thread_rng();
            match rng.gen_range(0..=4) {
                0 => Color::PaleGreen,
                1 => Color::PaleTurquoise,
                2 => Color::Red,
                3 => Color::Blue,
                _ => Color::Black,
            }
        }
        let mut output_bytes = Vec::new();
        {
            let mut writer = DotWriter::from(&mut output_bytes);
            writer.set_pretty_print(false);
            let mut digraph = writer.digraph();

            let mut threads: HashSet<usize> = HashSet::new();
            for node in &self.nodes {
                let thread_id = node.borrow().instruction.thread_id();
                threads.insert(thread_id);
            }

            for thread_id in threads {
                let mut cluster = digraph.cluster();
                cluster.set_color(get_color());
                cluster
                    .node_attributes()
                    .set_style(Style::Filled)
                    .set_color(Color::LightGrey);
                cluster.set_label(format!("Thread #{}", thread_id).as_str());
                for node in &self.nodes {
                    if node.borrow().instruction.thread_id() == thread_id {
                        let node_label = node.borrow().instruction.to_dot();
                        cluster.node_named(node_label.as_str());
                        for dependency in &node.borrow().depends_on {
                            let dependency_label = dependency.borrow().instruction.to_dot();
                            cluster.edge(node_label.as_str(), dependency_label.as_str());
                        }
                    }
                }
            }
        }
        String::from_utf8(output_bytes).unwrap()
    }
}
