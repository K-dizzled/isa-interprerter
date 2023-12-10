use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ArithCommand {
    Add,
    Sub,
    Mul,
    Div,
}

impl ArithCommand {
    pub fn apply(&self, lhs: usize, rhs: usize) -> usize {
        match self {
            Self::Add => lhs + rhs,
            Self::Sub => lhs - rhs,
            Self::Mul => lhs * rhs,
            Self::Div => lhs / rhs,
        }
    }
}

impl Display for ArithCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArithCommand::Add => write!(f, "+"),
            ArithCommand::Sub => write!(f, "-"),
            ArithCommand::Mul => write!(f, "*"),
            ArithCommand::Div => write!(f, "/"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MemoryAccessMode {
    SeqCst,
    Rel,
    Acq,
    RelAcq,
    Rlx,
}

impl Display for MemoryAccessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryAccessMode::SeqCst => write!(f, "SEQ_CST"),
            MemoryAccessMode::Rel => write!(f, "REL"),
            MemoryAccessMode::Acq => write!(f, "ACQ"),
            MemoryAccessMode::RelAcq => write!(f, "REL_ACQ"),
            MemoryAccessMode::Rlx => write!(f, "RLX"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    Register(String),
    Memory(String),
}

impl FromStr for Reference {
    type Err = ();

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        return match cmd.as_bytes() {
            [b'#', rest @ ..] => Ok(Self::Memory(std::str::from_utf8(rest).unwrap().to_string())),
            _ => Ok(Self::Register(cmd.to_string())),
        };
    }
}

impl Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reference::Register(reg) => write!(f, "r{}", reg),
            Reference::Memory(mem) => write!(f, "m{}", mem),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Arith(ArithCommand),
    Ref(Reference),
    Number(usize),
    MemoryAccess(MemoryAccessMode),
    Eq,
    Assign,
    Load,
    Store,
    If,
    Goto,
    Fence,
    Cas,
    Fai,
    Label(String),
}

#[derive(Debug)]
pub enum Error {
    InvalidCommand(String),
    InvalidInstruction(String),
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        return match cmd.as_bytes() {
            b"+" => Ok(ArithCommand::Add.into()),
            b"-" => Ok(ArithCommand::Sub.into()),
            b"/" => Ok(ArithCommand::Div.into()),
            b"*" => Ok(ArithCommand::Mul.into()),
            b"SEQ_CST" => Ok(MemoryAccessMode::SeqCst.into()),
            b"REL" => Ok(MemoryAccessMode::Rel.into()),
            b"ACQ" => Ok(MemoryAccessMode::Acq.into()),
            b"REL_ACQ" => Ok(MemoryAccessMode::RelAcq.into()),
            b"RLX" => Ok(MemoryAccessMode::Rlx.into()),
            b"=" => Ok(Self::Eq),
            b":=" => Ok(Self::Assign),
            b"load" => Ok(Self::Load),
            b"store" => Ok(Self::Store),
            b"if" => Ok(Self::If),
            b"goto" => Ok(Self::Goto),
            b"fence" => Ok(Self::Fence),
            b"cas" => Ok(Self::Cas),
            b"fai" => Ok(Self::Fai),
            reference if !reference.first().unwrap().is_ascii_digit() => Ok(Self::Ref(
                Reference::from_str(std::str::from_utf8(reference).unwrap())
                    .unwrap()
                    .into(),
            )),
            num => std::str::from_utf8(num)
                .unwrap()
                .parse::<usize>()
                .map(Self::Number)
                .map_err(|_| Error::InvalidCommand(cmd.to_string())),
        };
    }
}

impl From<ArithCommand> for Command {
    fn from(cmd: ArithCommand) -> Self {
        Self::Arith(cmd)
    }
}

impl From<MemoryAccessMode> for Command {
    fn from(cmd: MemoryAccessMode) -> Self {
        Self::MemoryAccess(cmd)
    }
}

pub struct WriteOperation {
    pub(crate) addr: String,
    pub(crate) value: usize,
}

impl WriteOperation {
    pub fn new(addr: String, value: usize) -> Self {
        Self { addr, value }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    AssignConst(Reference, usize),
    AssignOperation(Reference, Reference, ArithCommand, Reference),
    ConditionalJump(Reference, String),
    Load(MemoryAccessMode, Reference, Reference),
    Store(MemoryAccessMode, Reference, Reference),
    Cas(Reference, MemoryAccessMode, Reference, Reference, Reference),
    Fai(Reference, MemoryAccessMode, Reference, Reference),
    Fence(MemoryAccessMode),
}

impl Instruction {
    pub fn is_memory_access(&self) -> bool {
        match self {
            Self::Load(_, _, _) | Self::Store(_, _, _) => true,
            _ => false,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::AssignConst(dest, value) => {
                write!(f, "{} := {}", dest, value)
            }
            Instruction::AssignOperation(dest, lhs, op, rhs) => {
                write!(f, "{} := {} {} {}", dest, lhs, op, rhs)
            }
            Instruction::ConditionalJump(cond, label) => {
                write!(f, "if {} goto {}", cond, label)
            }
            Instruction::Load(mode, dest, addr) => {
                write!(f, "{} := load {} {}", dest, mode, addr)
            }
            Instruction::Store(mode, addr, value) => {
                write!(f, "store {} {} {}", mode, addr, value)
            }
            Instruction::Cas(dest, mode, addr, old, new) => {
                write!(f, "{} := cas {} {} {} {}", dest, mode, addr, old, new)
            }
            Instruction::Fai(dest, mode, addr, value) => {
                write!(f, "{} := fai {} {} {}", dest, mode, addr, value)
            }
            Instruction::Fence(mode) => {
                write!(f, "fence {}", mode)
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LabeledInstruction {
    pub label: Option<String>,
    pub instruction: Instruction,
    pub line_index: usize,
    pub thread_id: usize,
}

impl LabeledInstruction {
    pub fn new(
        label: Option<String>,
        instruction: Instruction,
        line_index: usize,
        thread_id: usize,
    ) -> Self {
        Self {
            label,
            instruction,
            line_index,
            thread_id,
        }
    }

    pub(crate) fn label(cmd: &str) -> (Option<String>, String) {
        let commands: Vec<&str> = cmd.split_whitespace().collect::<Vec<&str>>();
        let (label, commands) = if commands.first().unwrap().ends_with(':') {
            let label = commands.first().unwrap().replace(":", "").to_string();
            (Some(label), &commands[1..])
        } else {
            (None, commands.as_slice())
        };
        let cmd = commands.join(" ");
        (label, cmd.to_string())
    }

    pub fn from_line(line: &str, line_index: usize, thread_id: usize) -> Result<Self, Error> {
        let (label, cmd) = Self::label(line);
        let instruction = Instruction::from_str(cmd.as_str())?;
        Ok(Self::new(label, instruction, line_index, thread_id))
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.thread_id, self.line_index)
    }
}

impl FromStr for LabeledInstruction {
    type Err = Error;

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        let (label, cmd) = LabeledInstruction::label(cmd);
        let instruction = Instruction::from_str(cmd.as_str())?;
        Ok(Self {
            label,
            instruction,
            line_index: 0,
            thread_id: 0,
        })
    }
}

impl ToString for LabeledInstruction {
    fn to_string(&self) -> String {
        let label = match &self.label {
            Some(label) => format!("{}: ", label),
            None => "".to_string(),
        };

        format!(
            "Thread {}, line {}: {}{}",
            self.thread_id,
            self.line_index,
            label,
            self.instruction.to_string()
        )
    }
}

impl FromStr for Instruction {
    type Err = Error;

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        fn str_to_commands(cmd: &str) -> Vec<Command> {
            cmd.split_whitespace()
                .map(|cmd| cmd.parse::<Command>().unwrap())
                .collect()
        }

        let commands = str_to_commands(cmd);
        return match commands.as_slice() {
            [Command::Ref(ref1), Command::Eq, Command::Number(num)] => {
                Ok(Self::AssignConst(ref1.clone(), *num))
            }
            [Command::Ref(ref1), Command::Eq, Command::Ref(ref2), Command::Arith(cmd), Command::Ref(ref3)] => {
                Ok(Self::AssignOperation(
                    ref1.clone(),
                    ref2.clone(),
                    cmd.clone(),
                    ref3.clone(),
                ))
            }
            [Command::If, Command::Ref(ref1), Command::Goto, Command::Ref(Reference::Register(label))] => {
                Ok(Self::ConditionalJump(ref1.clone(), label.clone()))
            }
            [Command::Load, Command::MemoryAccess(mem_access), Command::Ref(addr), Command::Ref(reg)] => {
                Ok(Self::Load(mem_access.clone(), addr.clone(), reg.clone()))
            }
            [Command::Store, Command::MemoryAccess(mem_access), Command::Ref(addr), Command::Ref(reg)] => {
                Ok(Self::Store(mem_access.clone(), addr.clone(), reg.clone()))
            }
            [Command::Ref(ref1), Command::Assign, Command::Cas, Command::MemoryAccess(mem_access), Command::Ref(ref2), Command::Ref(ref3), Command::Ref(ref4)] => {
                Ok(Self::Cas(
                    ref1.clone(),
                    mem_access.clone(),
                    ref2.clone(),
                    ref3.clone(),
                    ref4.clone(),
                ))
            }
            [Command::Ref(ref1), Command::Assign, Command::Fai, Command::MemoryAccess(mem_access), Command::Ref(ref2), Command::Ref(ref3)] => {
                Ok(Self::Fai(
                    ref1.clone(),
                    mem_access.clone(),
                    ref2.clone(),
                    ref3.clone(),
                ))
            }
            [Command::Fence, Command::MemoryAccess(mem_access)] => {
                Ok(Self::Fence(mem_access.clone()))
            }
            _ => Err(Error::InvalidInstruction(cmd.to_string())),
        };
    }
}
