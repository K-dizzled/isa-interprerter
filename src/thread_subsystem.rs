use crate::dependency_graph::{DependencyGraph, InstructionNode, NodeType, Propagate};
use crate::instruction::{Instruction, LabeledInstruction, Reference};
use crate::memory_subsystem::{Memory, MemorySubsystem, SCMemorySubsystem, TSOMemorySubsystem};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;

pub struct Registers {
    pub registers: HashMap<usize, Memory>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    pub fn load(&self, addr: &str, thread_id: usize) -> usize {
        self.registers.get(&thread_id).unwrap().load(addr)
    }

    pub fn store(&mut self, addr: &str, value: usize, thread_id: usize) {
        self.registers
            .get_mut(&thread_id)
            .unwrap()
            .store(addr, value);
    }
}

impl std::fmt::Display for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (thread_id, memory) in self.registers.iter() {
            writeln!(f, "Thread {}", thread_id)?;
            writeln!(f, "{}", memory)?;
        }
        Ok(())
    }
}

pub struct TSO {
    pub memory_subsystem: TSOMemorySubsystem,
    pub programs: Vec<Vec<LabeledInstruction>>,
    pub dependency_graph: DependencyGraph,
    pub registers: Registers,
    pub is_pso: bool,
}

impl TSO {
    pub fn new(programs: Vec<Vec<LabeledInstruction>>, is_pso: bool) -> Self {
        let mut registers = Registers::new();
        let mut dependency_graph = DependencyGraph::new();
        for (thread_id, program) in programs.iter().enumerate() {
            for instruction in program.iter() {
                dependency_graph.add_node((*instruction).clone());
            }
            registers.registers.insert(thread_id, Memory::new());
        }
        dependency_graph.build_dependencies();

        Self {
            memory_subsystem: TSOMemorySubsystem::new(),
            programs,
            dependency_graph,
            registers,
            is_pso,
        }
    }

    pub fn get_instructions_to_exec(&self) -> Vec<Rc<RefCell<InstructionNode>>> {
        return self.dependency_graph.get_leaves();
    }

    pub fn save_graph(&self, filename: &str) {
        let file_content = self.dependency_graph.to_dot();
        let mut file = File::create(filename).expect("Unable to create file");
        file.write_all(file_content.as_bytes())
            .expect("Unable to write data");
    }

    pub fn exec_instruction(&mut self, instruction_node: Rc<RefCell<InstructionNode>>) {
        let instruction: NodeType = instruction_node.borrow_mut().instruction.clone();
        let thread_id = match instruction.borrow() {
            NodeType::Propagate(Propagate {
                associated_write, ..
            }) => associated_write.thread_id,
            NodeType::Instruction(labeled_instruction) => labeled_instruction.thread_id,
        };
        match instruction.clone() {
            NodeType::Propagate(Propagate { .. }) => {
                self.memory_subsystem.propagate(thread_id);
                self.dependency_graph
                    .remove_node(instruction_node.clone(), None, self.is_pso);
            }
            NodeType::Instruction(labeled_instruction) => match labeled_instruction
                .instruction
                .clone()
            {
                Instruction::AssignConst(Reference::Register(reg), value) => {
                    self.registers.store(reg.as_str(), value, thread_id);
                    self.dependency_graph
                        .remove_node(instruction_node.clone(), None, self.is_pso);
                }
                Instruction::AssignOperation(
                    Reference::Register(reg),
                    Reference::Register(reg1),
                    operation,
                    Reference::Register(reg2),
                ) => {
                    let value1 = self.registers.load(reg1.as_str(), thread_id);
                    let value2 = self.registers.load(reg2.as_str(), thread_id);

                    let result = operation.apply(value1, value2);
                    self.registers.store(reg.as_str(), result, thread_id);
                    self.dependency_graph
                        .remove_node(instruction_node.clone(), None, self.is_pso);
                }
                Instruction::Load(_, Reference::Memory(mem), Reference::Register(reg)) => {
                    let value = self.memory_subsystem.load(mem.as_str(), thread_id);
                    self.registers.store(reg.as_str(), value, thread_id);
                    self.dependency_graph
                        .remove_node(instruction_node.clone(), None, self.is_pso);
                }
                Instruction::Store(_, Reference::Register(reg), Reference::Memory(mem)) => {
                    let value = self.registers.load(reg.as_str(), thread_id);
                    self.memory_subsystem.store(mem.as_str(), value, thread_id);
                    if let Instruction::Store(_, _, mem_ref) =
                        labeled_instruction.instruction.clone()
                    {
                        let prop = (labeled_instruction.clone(), mem_ref.clone());
                        self.dependency_graph.remove_node(
                            instruction_node.clone(),
                            Some(prop),
                            self.is_pso,
                        );
                    } else {
                        panic!("Expected store instruction");
                    }
                }
                Instruction::Cas(
                    Reference::Register(ref1),
                    _,
                    Reference::Memory(addr),
                    Reference::Register(reg3),
                    Reference::Register(reg4),
                ) => {
                    let expected = self.registers.load(reg3.as_str(), thread_id);
                    let desired_set = self.registers.load(reg4.as_str(), thread_id);
                    let cur_value = self.memory_subsystem.load(addr.as_str(), thread_id);

                    if cur_value == expected {
                        self.memory_subsystem
                            .store(addr.as_str(), desired_set, thread_id);
                        self.registers.store(ref1.as_str(), cur_value, thread_id);

                        if let Instruction::Cas(_, _, mem_ref, _, _) =
                            labeled_instruction.instruction.clone()
                        {
                            let prop = (labeled_instruction.clone(), mem_ref.clone());
                            self.dependency_graph.remove_node(
                                instruction_node.clone(),
                                Some(prop),
                                self.is_pso,
                            );
                        } else {
                            panic!("Expected cas instruction");
                        }
                    } else {
                        self.registers.store(ref1.as_str(), cur_value, thread_id);
                        self.dependency_graph.remove_node(
                            instruction_node.clone(),
                            None,
                            self.is_pso,
                        );
                    }
                }
                Instruction::Fai(
                    Reference::Register(ref1),
                    _,
                    Reference::Memory(addr),
                    Reference::Register(reg3),
                ) => {
                    let prior_to_increment = self.memory_subsystem.load(addr.as_str(), thread_id);
                    let increment_by = self.registers.load(reg3.as_str(), thread_id);
                    let new_value = prior_to_increment + increment_by;

                    self.memory_subsystem
                        .store(addr.as_str(), new_value, thread_id);
                    self.registers
                        .store(ref1.as_str(), prior_to_increment, thread_id);

                    if let Instruction::Fai(_, _, mem_ref, _) =
                        labeled_instruction.instruction.clone()
                    {
                        let prop = (labeled_instruction.clone(), mem_ref.clone());
                        self.dependency_graph.remove_node(
                            instruction_node.clone(),
                            Some(prop),
                            self.is_pso,
                        );
                    } else {
                        panic!("Expected fai instruction");
                    }
                }
                Instruction::Fence(_) => {
                    self.dependency_graph
                        .remove_node(instruction_node.clone(), None, self.is_pso);
                }
                _ => {
                    panic!("Instruction not supported");
                }
            },
        }
    }
}

pub struct SequentialConsistency {
    pub memory_subsystem: SCMemorySubsystem,
    pub programs: Vec<Vec<LabeledInstruction>>,
    pub instruction_pointers: Vec<usize>,
    pub registers: Registers,
}

impl SequentialConsistency {
    pub fn new(programs: Vec<Vec<LabeledInstruction>>) -> Self {
        let mut registers = Registers::new();
        for (thread_id, _) in programs.iter().enumerate() {
            registers.registers.insert(thread_id, Memory::new());
        }
        Self {
            memory_subsystem: SCMemorySubsystem::new(),
            programs: programs.clone(),
            instruction_pointers: vec![0; programs.len()],
            registers,
        }
    }

    pub fn get_instructions_to_exec(&self) -> Vec<LabeledInstruction> {
        let mut instructions_to_exec = Vec::new();
        for (thread_id, program) in self.programs.iter().enumerate() {
            let instruction_pointer = self.instruction_pointers[thread_id];
            if instruction_pointer < program.len() {
                instructions_to_exec.push(program[instruction_pointer].clone());
            }
        }
        instructions_to_exec
    }

    fn find_label_index(&self, thread_id: usize, label: &str) -> usize {
        let program = &self.programs[thread_id];
        for (index, instruction) in program.iter().enumerate() {
            if let Some(labeled_label) = instruction.label.clone() {
                if labeled_label == label {
                    return index;
                }
            }
        }
        panic!("Label not found");
    }

    pub fn exec_instruction(&mut self, instruction: LabeledInstruction) {
        let thread_id = instruction.thread_id;
        match instruction.instruction {
            Instruction::AssignConst(Reference::Register(reg), value) => {
                self.registers.store(reg.as_str(), value, thread_id);
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::AssignOperation(
                Reference::Register(reg),
                Reference::Register(reg1),
                operation,
                Reference::Register(reg2),
            ) => {
                let value1 = self.registers.load(reg1.as_str(), thread_id);
                let value2 = self.registers.load(reg2.as_str(), thread_id);

                let result = operation.apply(value1, value2);
                self.registers.store(reg.as_str(), result, thread_id);
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::Load(_, Reference::Memory(mem), Reference::Register(reg)) => {
                let value = self.memory_subsystem.load(mem.as_str(), thread_id);
                self.registers.store(reg.as_str(), value, thread_id);
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::Store(_, Reference::Register(reg), Reference::Memory(mem)) => {
                let value = self.registers.load(reg.as_str(), thread_id);
                self.memory_subsystem.store(mem.as_str(), value, thread_id);
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::Cas(
                Reference::Register(ref1),
                _,
                Reference::Memory(addr),
                Reference::Register(reg3),
                Reference::Register(reg4),
            ) => {
                let expected = self.registers.load(reg3.as_str(), thread_id);
                let desired_set = self.registers.load(reg4.as_str(), thread_id);
                let cur_value = self.memory_subsystem.load(addr.as_str(), thread_id);

                if cur_value == expected {
                    self.memory_subsystem
                        .store(addr.as_str(), desired_set, thread_id);
                    self.registers.store(ref1.as_str(), cur_value, thread_id);
                } else {
                    self.registers.store(ref1.as_str(), cur_value, thread_id);
                }
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::Fai(
                Reference::Register(ref1),
                _,
                Reference::Memory(addr),
                Reference::Register(reg3),
            ) => {
                let prior_to_increment = self.memory_subsystem.load(addr.as_str(), thread_id);
                let increment_by = self.registers.load(reg3.as_str(), thread_id);
                let new_value = prior_to_increment + increment_by;

                self.memory_subsystem
                    .store(addr.as_str(), new_value, thread_id);
                self.registers
                    .store(ref1.as_str(), prior_to_increment, thread_id);
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::Fence(_) => {
                self.instruction_pointers[thread_id] += 1;
            }
            Instruction::ConditionalJump(Reference::Register(reg), label) => {
                let value = self.registers.load(reg.as_str(), thread_id);
                if value != 0 {
                    let label_index = self.find_label_index(thread_id, label.as_str());
                    self.instruction_pointers[thread_id] = label_index;
                } else {
                    self.instruction_pointers[thread_id] += 1;
                }
            }
            _ => {
                panic!("Instruction not supported");
            }
        }
    }
}
