mod dependency_graph;
mod instruction;
mod memory_subsystem;
mod thread_subsystem;
mod utils;

use crate::dependency_graph::InstructionNode;
pub use instruction::{
    ArithCommand, Command, Error, Instruction, LabeledInstruction, MemoryAccessMode, Reference,
};
pub use memory_subsystem::Memory;
use std::cell::RefCell;
use std::rc::Rc;
pub use thread_subsystem::{SequentialConsistency, TSO};
pub use utils::programs_to_instructions;

pub struct InterpretorSC {
    system: SequentialConsistency,
}

impl InterpretorSC {
    pub fn new(program_paths: Vec<String>) -> Self {
        let instructions = programs_to_instructions(program_paths);
        Self {
            system: SequentialConsistency::new(instructions),
        }
    }

    pub fn run(&mut self) {
        loop {
            let options = self.system.get_instructions_to_exec();
            if options.is_empty() {
                println!("No more instructions to execute");
                break;
            }
            for (index, option) in options.iter().enumerate() {
                println!("{} | {}", index, option.to_string());
            }
            println!("Please select an option and input the index: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim() == "exit" {
                break;
            } else if input.trim() == "registers" {
                println!("{}", self.system.registers);
                continue;
            } else if input.trim() == "memory" {
                println!("{}", self.system.memory_subsystem.memory);
                continue;
            }
            let index: usize = input
                .trim()
                .parse::<usize>()
                .expect("Invalid command or index");
            if index >= options.len() {
                println!("Invalid index");
                continue;
            }
            let option: LabeledInstruction = options[index].clone();
            self.system.exec_instruction(option);
        }
    }
}

pub struct InterpretorTSO {
    system: TSO,
}

impl InterpretorTSO {
    pub fn new(program_paths: Vec<String>, is_pso: bool) -> Self {
        let instructions = programs_to_instructions(program_paths);
        Self {
            system: TSO::new(instructions, is_pso),
        }
    }

    pub fn run(&mut self) {
        loop {
            let options = self.system.get_instructions_to_exec();
            if options.is_empty() {
                println!("No more instructions to execute");
                break;
            }
            for (index, option) in options.iter().enumerate() {
                println!("{} | {}", index, option.borrow().instruction.to_string());
            }
            println!("Please select an option and input the index: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim() == "exit" {
                break;
            } else if input.trim() == "registers" {
                println!("{}", self.system.registers);
                continue;
            } else if input.trim() == "memory" {
                println!("{}", self.system.memory_subsystem.memory);
                continue;
            } else if input.starts_with("graph") {
                let path = input.trim().split(" ").collect::<Vec<&str>>()[1];
                self.system.save_graph(path);
                continue;
            }
            let index: usize = input
                .trim()
                .parse::<usize>()
                .expect("Invalid command or index");
            if index >= options.len() {
                println!("Invalid index");
                continue;
            }
            let option: Rc<RefCell<InstructionNode>> = options[index].clone();
            self.system.exec_instruction(option);
        }
    }
}
