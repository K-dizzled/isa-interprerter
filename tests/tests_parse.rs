use isa_interpreter::{ArithCommand, Instruction, MemoryAccessMode, Reference};

use pretty_assertions::assert_eq;

#[test]
fn test_assign_const() {
    let instr = "x = 1";
    let expected = Instruction::AssignConst(Reference::Register("x".to_string()), 1);
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_binary_op() {
    let instr = "x = r1 + r2";
    let expected = Instruction::AssignOperation(
        Reference::Register("x".to_string()),
        Reference::Register("r1".to_string()),
        ArithCommand::Add,
        Reference::Register("r2".to_string()),
    );
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_conditional_jump() {
    let instr = "if r1 goto L5";
    let expected =
        Instruction::ConditionalJump(Reference::Register("r1".to_string()), "L5".to_string());
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_load() {
    let instr = "load SEQ_CST #r1 r2";
    let expected = Instruction::Load(
        MemoryAccessMode::SeqCst,
        Reference::Memory("r1".to_string()),
        Reference::Register("r2".to_string()),
    );
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_store() {
    let instr = "store RLX r1 #r2";
    let expected = Instruction::Store(
        MemoryAccessMode::Rlx,
        Reference::Register("r1".to_string()),
        Reference::Memory("r2".to_string()),
    );
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_cas() {
    let instr = "r1 := cas REL #r2 r3 r4";
    let expected = Instruction::Cas(
        Reference::Register("r1".to_string()),
        MemoryAccessMode::Rel,
        Reference::Memory("r2".to_string()),
        Reference::Register("r3".to_string()),
        Reference::Register("r4".to_string()),
    );
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_fai() {
    let instr = "r1 := fai ACQ #r2 r3";
    let expected = Instruction::Fai(
        Reference::Register("r1".to_string()),
        MemoryAccessMode::Acq,
        Reference::Memory("r2".to_string()),
        Reference::Register("r3".to_string()),
    );
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}

#[test]
fn test_fence() {
    let instr = "fence REL_ACQ";
    let expected = Instruction::Fence(MemoryAccessMode::RelAcq);
    assert_eq!(expected, instr.parse::<Instruction>().unwrap());
}
