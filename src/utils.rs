use crate::instruction::LabeledInstruction;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn parse_program(file_path: String, thread_id: usize) -> Vec<LabeledInstruction> {
    let file = File::open(file_path.clone()).unwrap();
    let reader = BufReader::new(file);
    let mut program = Vec::new();
    for line in reader.lines() {
        if let Ok(instruction) = line {
            let instruction: String = instruction.trim().to_string();
            if instruction.is_empty() {
                continue;
            }
            let parsed: LabeledInstruction = instruction
                .parse::<LabeledInstruction>()
                .expect(format!("Invalid instruction found in {}", file_path).as_str());
            let labeled_instruction =
                LabeledInstruction::new(parsed.label, parsed.instruction, program.len(), thread_id);
            program.push(labeled_instruction);
        }
    }
    program
}

pub fn programs_to_instructions(file_paths: Vec<String>) -> Vec<Vec<LabeledInstruction>> {
    let mut programs = Vec::new();
    for (thread_id, file_path) in file_paths.iter().enumerate() {
        let program = parse_program(file_path.to_string(), thread_id);
        programs.push(program);
    }
    programs
}
