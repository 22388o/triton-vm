use super::ord_n::{Ord16, Ord16::*, Ord7};
use num_traits::One;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::ops::Neg;
use std::str::SplitWhitespace;
use twenty_first::shared_math::b_field_element::BFieldElement;
use AnInstruction::*;
use TokenError::*;

/// An `Instruction` has `call` addresses encoded as absolute integers.
pub type Instruction = AnInstruction<BFieldElement>;

/// A `LabelledInstruction` has `call` addresses encoded as label names.
///
/// A label name is a `String` that occurs as "`label_name`:".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelledInstruction {
    Instruction(AnInstruction<String>),
    Label(String),
}

impl Display for LabelledInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelledInstruction::Instruction(instr) => write!(f, "{}", instr),
            LabelledInstruction::Label(label_name) => write!(f, "{}:", label_name),
        }
    }
}

/// A Triton VM instruction
///
/// The ISA is defined at:
///
/// https://neptune.builders/core-team/triton-vm/src/branch/master/specification/isa.md
///
/// The type parameter `Dest` describes the type of addresses (absolute or labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnInstruction<Dest> {
    // OpStack manipulation
    Pop,
    Push(BFieldElement),
    Divine,
    Dup(Ord16),
    Swap(Ord16),

    // Control flow
    Nop,
    Skiz,
    Call(Dest),
    Return,
    Recurse,
    Assert,
    Halt,

    // Memory access
    ReadMem,
    WriteMem,

    // Hashing-related instructions
    Hash,
    DivineSibling,
    AssertVector,

    // Arithmetic on stack instructions
    Add,
    Mul,
    Invert,
    Split,
    Eq,
    Lt,
    And,
    Xor,
    Reverse,
    Div,
    XxAdd,
    XxMul,
    XInvert,
    XbMul,

    // Read/write
    ReadIo,
    WriteIo,
}

impl<Dest: Display> Display for AnInstruction<Dest> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // OpStack manipulation
            Pop => write!(f, "pop"),
            Push(arg) => write!(f, "push {}", arg),
            Divine => write!(f, "divine"),
            Dup(arg) => write!(f, "dup{}", arg),
            Swap(arg) => write!(f, "swap{}", arg),
            // Control flow
            Nop => write!(f, "nop"),
            Skiz => write!(f, "skiz"),
            Call(arg) => write!(f, "call {}", arg),
            Return => write!(f, "return"),
            Recurse => write!(f, "recurse"),
            Assert => write!(f, "assert"),
            Halt => write!(f, "halt"),

            // Memory access
            ReadMem => write!(f, "read_mem"),
            WriteMem => write!(f, "write_mem"),

            // Hash instructions
            Hash => write!(f, "hash"),
            DivineSibling => write!(f, "divine_sibling"),
            AssertVector => write!(f, "assert_vector"),

            // Arithmetic on stack instructions
            Add => write!(f, "add"),
            Mul => write!(f, "mul"),
            Invert => write!(f, "invert"),
            Split => write!(f, "split"),
            Eq => write!(f, "eq"),
            Lt => write!(f, "lt"),
            And => write!(f, "and"),
            Xor => write!(f, "xor"),
            Reverse => write!(f, "reverse"),
            Div => write!(f, "div"),
            XxAdd => write!(f, "xxadd"),
            XxMul => write!(f, "xxmul"),
            XInvert => write!(f, "xinvert"),
            XbMul => write!(f, "xbmul"),

            // Read/write
            ReadIo => write!(f, "read_io"),
            WriteIo => write!(f, "write_io"),
        }
    }
}

impl<Dest> AnInstruction<Dest> {
    /// Assign a unique positive integer to each `Instruction`.
    pub fn opcode(&self) -> u32 {
        match self {
            // OpStack manipulation
            Pop => 2,
            Push(_) => 1,
            Divine => 4,
            Dup(_) => 5,
            Swap(_) => 9,

            // Control flow
            Nop => 8,
            Skiz => 6,
            Call(_) => 13,
            Return => 12,
            Recurse => 16,
            Assert => 10,
            Halt => 0,

            // Memory access
            ReadMem => 20,
            WriteMem => 24,

            // Hashing-related instructions
            Hash => 28,
            DivineSibling => 32,
            AssertVector => 36,

            // Arithmetic on stack instructions
            Add => 14,
            Mul => 18,
            Invert => 40,
            Split => 44,
            Eq => 22,
            Lt => 26,
            And => 30,
            Xor => 34,
            Reverse => 48,
            Div => 52,

            XxAdd => 56,
            XxMul => 60,
            XInvert => 64,
            XbMul => 38,

            // Read/write
            ReadIo => 68,
            WriteIo => 42,
        }
    }

    /// Returns whether a given instruction modifies the op-stack.
    ///
    /// A modification involves any amount of pushing and/or popping.
    pub fn is_op_stack_instruction(&self) -> bool {
        !matches!(
            self,
            Nop | Call(_) | Return | Recurse | Halt | Hash | AssertVector
        )
    }

    pub fn is_u32_op(&self) -> bool {
        matches!(self, Lt | And | Xor | Reverse | Div)
    }

    pub fn opcode_b(&self) -> BFieldElement {
        self.opcode().into()
    }

    pub fn size(&self) -> usize {
        if matches!(self, Push(_) | Dup(_) | Swap(_) | Call(_)) {
            2
        } else {
            1
        }
    }

    /// Get the i'th instruction bit
    pub fn ib(&self, arg: Ord7) -> BFieldElement {
        let opcode = self.opcode();
        let bit_number: usize = arg.into();

        ((opcode >> bit_number) & 1).into()
    }

    fn map_call_address<F, NewDest>(&self, f: F) -> AnInstruction<NewDest>
    where
        F: Fn(&Dest) -> NewDest,
    {
        match self {
            Pop => Pop,
            Push(x) => Push(*x),
            Divine => Divine,
            Dup(x) => Dup(*x),
            Swap(x) => Swap(*x),
            Nop => Nop,
            Skiz => Skiz,
            Call(label) => Call(f(label)),
            Return => Return,
            Recurse => Recurse,
            Assert => Assert,
            Halt => Halt,
            ReadMem => ReadMem,
            WriteMem => WriteMem,
            Hash => Hash,
            DivineSibling => DivineSibling,
            AssertVector => AssertVector,
            Add => Add,
            Mul => Mul,
            Invert => Invert,
            Split => Split,
            Eq => Eq,
            Lt => Lt,
            And => And,
            Xor => Xor,
            Reverse => Reverse,
            Div => Div,
            XxAdd => XxAdd,
            XxMul => XxMul,
            XInvert => XInvert,
            XbMul => XbMul,
            ReadIo => ReadIo,
            WriteIo => WriteIo,
        }
    }
}

impl Instruction {
    pub fn arg(&self) -> Option<BFieldElement> {
        match self {
            // Double-word instructions (instructions that take arguments)
            Push(arg) => Some(*arg),
            Dup(arg) => Some(ord16_to_bfe(arg)),
            Swap(arg) => Some(ord16_to_bfe(arg)),
            Call(arg) => Some(*arg),
            _ => None,
        }
    }
}

impl TryFrom<u32> for Instruction {
    type Error = String;

    fn try_from(opcode: u32) -> Result<Self, Self::Error> {
        match opcode {
            2 => Ok(Pop),
            1 => Ok(Push(Default::default())),
            4 => Ok(Divine),
            5 => Ok(Dup(ST0)),
            9 => Ok(Swap(ST0)),
            8 => Ok(Nop),
            6 => Ok(Skiz),
            13 => Ok(Call(Default::default())),
            12 => Ok(Return),
            16 => Ok(Recurse),
            10 => Ok(Assert),
            0 => Ok(Halt),
            20 => Ok(ReadMem),
            24 => Ok(WriteMem),
            28 => Ok(Hash),
            32 => Ok(DivineSibling),
            36 => Ok(AssertVector),
            14 => Ok(Add),
            18 => Ok(Mul),
            40 => Ok(Invert),
            44 => Ok(Split),
            22 => Ok(Eq),
            26 => Ok(Lt),
            30 => Ok(And),
            34 => Ok(Xor),
            48 => Ok(Reverse),
            52 => Ok(Div),
            56 => Ok(XxAdd),
            60 => Ok(XxMul),
            64 => Ok(XInvert),
            38 => Ok(XbMul),
            68 => Ok(ReadIo),
            42 => Ok(WriteIo),
            _ => Err(format!("No instruction with opcode {} exists.", opcode)),
        }
    }
}

impl TryFrom<u64> for Instruction {
    type Error = String;

    fn try_from(opcode: u64) -> Result<Self, Self::Error> {
        (opcode as u32).try_into()
    }
}

impl TryFrom<usize> for Instruction {
    type Error = String;

    fn try_from(opcode: usize) -> Result<Self, Self::Error> {
        (opcode as u32).try_into()
    }
}

fn ord16_to_bfe(n: &Ord16) -> BFieldElement {
    let n: u32 = n.into();
    n.into()
}

#[derive(Debug)]
pub enum TokenError {
    UnexpectedEndOfStream,
    UnknownInstruction(String),
}

impl Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnknownInstruction(s) => write!(f, "UnknownInstruction({})", s),
            UnexpectedEndOfStream => write!(f, "UnexpectedEndOfStream"),
        }
    }
}

impl Error for TokenError {}

/// Convert a program with labels to a program with absolute positions
pub fn convert_labels(program: &[LabelledInstruction]) -> Vec<Instruction> {
    let mut label_map = HashMap::<String, usize>::new();
    let mut instruction_pointer: usize = 0;

    // 1. Add all labels to a map
    for labelled_instruction in program.iter() {
        match labelled_instruction {
            LabelledInstruction::Label(label_name) => {
                label_map.insert(label_name.clone(), instruction_pointer);
            }

            LabelledInstruction::Instruction(instr) => {
                instruction_pointer += instr.size();
            }
        }
    }

    // 2. Convert every label to the lookup value of that map
    program
        .iter()
        .flat_map(|labelled_instruction| convert_labels_helper(labelled_instruction, &label_map))
        .collect()
}

fn convert_labels_helper(
    instruction: &LabelledInstruction,
    label_map: &HashMap<String, usize>,
) -> Vec<Instruction> {
    match instruction {
        LabelledInstruction::Label(_) => vec![],

        LabelledInstruction::Instruction(instr) => {
            let unlabelled_instruction: Instruction = instr.map_call_address(|label_name| {
                // FIXME: Consider failing graciously on missing labels.
                let label_not_found = format!("Label not found: {}", label_name);
                let absolute_address = label_map.get(label_name).expect(&label_not_found);
                BFieldElement::new(*absolute_address as u64)
            });

            vec![unlabelled_instruction]
        }
    }
}

pub fn parse(code: &str) -> Result<Vec<LabelledInstruction>, Box<dyn Error>> {
    let mut tokens = code.split_whitespace();
    let mut instructions = vec![];

    while let Some(token) = tokens.next() {
        let mut instruction = parse_token(token, &mut tokens)?;
        instructions.append(&mut instruction);
    }

    Ok(instructions)
}

fn parse_token(
    token: &str,
    tokens: &mut SplitWhitespace,
) -> Result<Vec<LabelledInstruction>, Box<dyn Error>> {
    if let Some(label) = token.strip_suffix(':') {
        let label_name = label.to_string();
        return Ok(vec![LabelledInstruction::Label(label_name)]);
    }

    let instruction: Vec<AnInstruction<String>> = match token {
        // OpStack manipulation
        "pop" => vec![Pop],
        "push" => vec![Push(parse_elem(tokens)?)],
        "divine" => vec![Divine],
        "dup0" => vec![Dup(ST0)],
        "dup1" => vec![Dup(ST1)],
        "dup2" => vec![Dup(ST2)],
        "dup3" => vec![Dup(ST3)],
        "dup4" => vec![Dup(ST4)],
        "dup5" => vec![Dup(ST5)],
        "dup6" => vec![Dup(ST6)],
        "dup7" => vec![Dup(ST7)],
        "dup8" => vec![Dup(ST8)],
        "dup9" => vec![Dup(ST9)],
        "dup10" => vec![Dup(ST10)],
        "dup11" => vec![Dup(ST11)],
        "dup12" => vec![Dup(ST12)],
        "dup13" => vec![Dup(ST13)],
        "dup14" => vec![Dup(ST14)],
        "dup15" => vec![Dup(ST15)],
        "swap1" => vec![Swap(ST1)],
        "swap2" => vec![Swap(ST2)],
        "swap3" => vec![Swap(ST3)],
        "swap4" => vec![Swap(ST4)],
        "swap5" => vec![Swap(ST5)],
        "swap6" => vec![Swap(ST6)],
        "swap7" => vec![Swap(ST7)],
        "swap8" => vec![Swap(ST8)],
        "swap9" => vec![Swap(ST9)],
        "swap10" => vec![Swap(ST10)],
        "swap11" => vec![Swap(ST11)],
        "swap12" => vec![Swap(ST12)],
        "swap13" => vec![Swap(ST13)],
        "swap14" => vec![Swap(ST14)],
        "swap15" => vec![Swap(ST15)],

        // Control flow
        "nop" => vec![Nop],
        "skiz" => vec![Skiz],
        "call" => vec![Call(parse_label(tokens)?)],
        "return" => vec![Return],
        "recurse" => vec![Recurse],
        "assert" => vec![Assert],
        "halt" => vec![Halt],

        // Memory access
        "read_mem" => vec![ReadMem],
        "write_mem" => vec![WriteMem],

        // Hashing-related instructions
        "hash" => vec![Hash],
        "divine_sibling" => vec![DivineSibling],
        "assert_vector" => vec![AssertVector],

        // Arithmetic on stack instructions
        "add" => vec![Add],
        "mul" => vec![Mul],
        "invert" => vec![Invert],
        "split" => vec![Split],
        "eq" => vec![Eq],
        "lt" => vec![Lt],
        "and" => vec![And],
        "xor" => vec![Xor],
        "reverse" => vec![Reverse],
        "div" => vec![Div],
        "xxadd" => vec![XxAdd],
        "xxmul" => vec![XxMul],
        "xinvert" => vec![XInvert],
        "xbmul" => vec![XbMul],

        // Pseudo-instructions
        "neg" => vec![Push(BFieldElement::one().neg()), Mul],
        "sub" => vec![Swap(ST1), Push(BFieldElement::one().neg()), Mul, Add],

        // Read/write
        "read_io" => vec![ReadIo],
        "write_io" => vec![WriteIo],

        _ => return Err(Box::new(UnknownInstruction(token.to_string()))),
    };

    let labelled_instruction = instruction
        .into_iter()
        .map(LabelledInstruction::Instruction)
        .collect();

    Ok(labelled_instruction)
}

fn parse_elem(tokens: &mut SplitWhitespace) -> Result<BFieldElement, Box<dyn Error>> {
    let constant_s = tokens.next().ok_or(UnexpectedEndOfStream)?;

    let mut constant_n128: i128 = constant_s.parse::<i128>()?;
    if constant_n128 < 0 {
        constant_n128 += BFieldElement::QUOTIENT as i128;
    }
    let constant_n64: u64 = constant_n128.try_into()?;
    let constant_elem = BFieldElement::new(constant_n64);

    Ok(constant_elem)
}

fn parse_label(tokens: &mut SplitWhitespace) -> Result<String, Box<dyn Error>> {
    let label = tokens
        .next()
        .map(|s| s.to_string())
        .ok_or(UnexpectedEndOfStream)?;

    Ok(label)
}

pub fn all_instructions_without_args() -> Vec<Instruction> {
    vec![
        Pop,
        Push(Default::default()),
        Divine,
        Dup(Default::default()),
        Swap(Default::default()),
        Nop,
        Skiz,
        Call(Default::default()),
        Return,
        Recurse,
        Assert,
        Halt,
        ReadMem,
        WriteMem,
        Hash,
        DivineSibling,
        AssertVector,
        Add,
        Mul,
        Invert,
        Split,
        Eq,
        Lt,
        And,
        Xor,
        Reverse,
        Div,
        XxAdd,
        XxMul,
        XInvert,
        XbMul,
        ReadIo,
        WriteIo,
    ]
}

pub fn all_labelled_instructions_with_args() -> Vec<LabelledInstruction> {
    vec![
        Pop,
        Push(BFieldElement::new(42)),
        Divine,
        Dup(ST0),
        Dup(ST1),
        Dup(ST2),
        Dup(ST3),
        Dup(ST4),
        Dup(ST5),
        Dup(ST6),
        Dup(ST7),
        Dup(ST8),
        Dup(ST9),
        Dup(ST10),
        Dup(ST11),
        Dup(ST12),
        Dup(ST13),
        Dup(ST14),
        Dup(ST15),
        Swap(ST1),
        Swap(ST2),
        Swap(ST3),
        Swap(ST4),
        Swap(ST5),
        Swap(ST6),
        Swap(ST7),
        Swap(ST8),
        Swap(ST9),
        Swap(ST10),
        Swap(ST11),
        Swap(ST12),
        Swap(ST13),
        Swap(ST14),
        Swap(ST15),
        Nop,
        Skiz,
        Call("foo".to_string()),
        Return,
        Recurse,
        Assert,
        Halt,
        ReadMem,
        WriteMem,
        Hash,
        DivineSibling,
        AssertVector,
        Add,
        Mul,
        Invert,
        Split,
        Eq,
        Lt,
        And,
        Xor,
        Reverse,
        Div,
        XxAdd,
        XxMul,
        XInvert,
        XbMul,
        ReadIo,
        WriteIo,
    ]
    .into_iter()
    .map(LabelledInstruction::Instruction)
    .collect()
}

pub mod sample_programs {
    use twenty_first::shared_math::b_field_element::BFieldElement;

    use super::super::vm::Program;
    use super::{AnInstruction::*, LabelledInstruction};

    pub const PUSH_PUSH_ADD_POP_S: &str = "
        push 1
        push 1
        add
        pop
    ";

    pub fn push_push_add_pop_p() -> Program {
        let instructions: Vec<LabelledInstruction> = vec![
            Push(BFieldElement::new(1)),
            Push(BFieldElement::new(1)),
            Add,
            Pop,
        ]
        .into_iter()
        .map(LabelledInstruction::Instruction)
        .collect();

        Program::new(&instructions)
    }

    /// TVM assembly to sample weights for the recursive verifier
    ///
    /// input: seed, num_weights
    ///
    /// output: num_weights-many random weights
    pub const SAMPLE_WEIGHTS: &str = concat!(
        "push 17 push 13 push 11 ",     // get seed – should be an argument
        "read_io ",                     // number of weights – should be argument
        "sample_weights: ",             // proper program starts here
        "call sample_weights_loop ",    // setup done, start sampling loop
        "pop pop ",                     // clean up stack: RAM value & pointer
        "pop pop pop pop ",             // clean up stack: seed & countdown
        "halt ",                        // done – should be return
        "",                             //
        "sample_weights_loop: ",        // subroutine: loop until all weights are sampled
        "dup0 push 0 eq skiz return ",  // no weights left
        "push -1 add ",                 // decrease number of weights to still sample
        "push 0 push 0 push 0 push 0 ", // prepare for hashing
        "push 0 push 0 push 0 push 0 ", // prepare for hashing
        "dup11 dup11 dup11 dup11 ",     // prepare for hashing
        "hash ",                        // hash seed & countdown
        "swap13 swap10 pop ",           // re-organize stack
        "swap13 swap10 pop ",           // re-organize stack
        "swap13 swap10 swap7 ",         // re-organize stack
        "pop pop pop pop pop pop pop ", // remove unnecessary remnants of digest
        "recurse ",                     // repeat
    );

    /// TVM assembly to verify Merkle authentication paths
    ///
    /// input: merkle root, number of leafs, leaf values, APs
    ///
    /// output: Result<(), VMFail>
    pub const MT_AP_VERIFY: &str = concat!(
        "read_io ",                                 // number of authentication paths to test
        "",                                         // stack: [num]
        "mt_ap_verify: ",                           // proper program starts here
        "push 0 swap1 write_mem pop pop ",          // store number of APs at RAM address 0
        "",                                         // stack: []
        "read_io read_io read_io read_io read_io ", // read Merkle root
        "",                                         // stack: [r4 r3 r2 r1 r0]
        "call check_aps ",                          //
        "pop pop pop pop pop ",                     // leave clean stack: Merkle root
        "",                                         // stack: []
        "halt ",                                    // done – should be “return”
        "",
        "",                               // subroutine: check AP one at a time
        "",                               // stack before: [* r4 r3 r2 r1 r0]
        "",                               // stack after: [* r4 r3 r2 r1 r0]
        "check_aps: ",                    // start function description:
        "push 0 push 0 read_mem dup0 ",   // get number of APs left to check
        "",                               // stack: [* r4 r3 r2 r1 r0 0 num_left num_left]
        "push 0 eq ",                     // see if there are authentication paths left
        "",                               // stack: [* r4 r3 r2 r1 r0 0 num_left num_left==0]
        "skiz return ",                   // return if no authentication paths left
        "push -1 add write_mem pop pop ", // decrease number of authentication paths left to check
        "",                               // stack: [* r4 r3 r2 r1 r0]
        "call get_idx_and_hash_leaf ",    //
        "",                               // stack: [* r4 r3 r2 r1 r0 idx d4 d3 d2 d1 d0 0 0 0 0 0]
        "call traverse_tree ",            //
        "",                               // stack: [* r4 r3 r2 r1 r0 idx>>2 - - - - - - - - - -]
        "call assert_tree_top ",          //
        // stack: [* r4 r3 r2 r1 r0]
        "recurse ", // check next AP
        "",
        "",                                         // subroutine: read index & hash leaf
        "",                                         // stack before: [*]
        "",                        // stack afterwards: [* idx d4 d3 d2 d1 d0 0 0 0 0 0]
        "get_idx_and_hash_leaf: ", // start function description:
        "read_io ",                // read node index
        "read_io read_io read_io read_io read_io ", // read leaf's value
        "push 0 push 0 push 0 push 0 push 0 ", // pad before fixed-length hash
        "hash return ",            // compute leaf's digest
        "",
        "",                             // subroutine: go up tree
        "",                             // stack before: [* idx - - - - - - - - - -]
        "",                             // stack after: [* idx>>2 - - - - - - - - - -]
        "traverse_tree: ",              // start function description:
        "dup10 push 1 eq skiz return ", // break loop if node index is 1
        "divine_sibling hash recurse ", // move up one level in the Merkle tree
        "",
        "",                     // subroutine: compare digests
        "",                     // stack before: [* r4 r3 r2 r1 r0 idx a b c d e - - - - -]
        "",                     // stack after: [* r4 r3 r2 r1 r0]
        "assert_tree_top: ",    // start function description:
        "pop pop pop pop pop ", // remove unnecessary “0”s from hashing
        "",                     // stack: [* r4 r3 r2 r1 r0 idx a b c d e]
        "swap1 swap2 swap3 swap4 swap5 ",
        "",                     // stack: [* r4 r3 r2 r1 r0 a b c d e idx]
        "assert ",              //
        "",                     // stack: [* r4 r3 r2 r1 r0 a b c d e]
        "assert_vector ",       // actually compare to root of tree
        "pop pop pop pop pop ", // clean up stack, leave only one root
        "return ",              //
    );

    // see also: get_colinear_y in src/shared_math/polynomial.rs
    pub const GET_COLINEAR_Y: &str = concat!(
        "read_io ",                       // p2_x
        "read_io read_io ",               // p1_y p1_x
        "read_io read_io ",               // p0_y p0_x
        "swap3 push -1 mul dup1 add ",    // dy = p0_y - p1_y
        "dup3 push -1 mul dup5 add mul ", // dy·(p2_x - p0_x)
        "dup3 dup3 push -1 mul add ",     // dx = p0_x - p1_x
        "invert mul add ",                // compute result
        "swap3 pop pop pop ",             // leave a clean stack
        "write_io halt ",
    );

    pub const HELLO_WORLD_1: &str = "
        push 10
        push 33
        push 100
        push 108
        push 114
        push 111
        push 87
        push 32
        push 44
        push 111
        push 108
        push 108
        push 101
        push 72

        write_io write_io write_io write_io write_io write_io write_io
        write_io write_io write_io write_io write_io write_io write_io
        ";

    pub const BASIC_RAM_READ_WRITE: &str = concat!(
        "push  5 push  6 write_mem pop pop ",
        "push 15 push 16 write_mem pop pop ",
        "push  5 push  0 read_mem  pop pop ",
        "push 15 push  0 read_mem  pop pop ",
        "push  5 push  7 write_mem pop pop ",
        "push 15 push  0 read_mem ",
        "push  5 push  0 read_mem ",
        "halt ",
    );

    pub const EDGY_RAM_WRITES: &str = concat!(
        "write_mem ",                         // this should write 0 to address 0
        "push 5 swap2 push 3 swap2 pop pop ", // stack is now of length 16 again
        "write_mem ",                         // this should write 3 to address 5
        "swap2 read_mem ",                    // stack's top should now be 3, 5, 3, 0, 0, …
        "halt ",
    );

    pub const READ_WRITE_X3: &str = "
        read_io
        read_io
        read_io
        write_io
        write_io
        write_io
    ";

    pub const READ_X3_WRITE_X14: &str = "
        read_io read_io read_io
        dup0 dup2 dup4
        dup0 dup2 dup4
        dup0 dup2 dup4
        dup0 dup2
        write_io write_io write_io write_io
        write_io write_io write_io write_io
        write_io write_io write_io write_io
        write_io write_io
    ";

    pub const COUNTDOWN_FROM_10: &str = "
        push 10
        call foo
   foo: push 18446744069414584320
        add
        dup0
        skiz
        recurse
        halt
    ";

    // leave the stack with the n first fibonacci numbers.  f_0 = 0; f_1 = 1
    // buttom-up approach
    pub const FIBONACCI_SOURCE: &str = "
    push 0
    push 1
    push n=6
    -- case: n==0 || n== 1
    dup0
    dup0
    dup0
    mul
    eq
    skiz
    call $basecase
    -- case: n>1
    call $nextline
    call $fib
    swap1 - n on top
    push 18446744069414584320
    add
    skiz
    recurse
    call $basecase
    dup0     :basecase
    push 0
    eq
    skiz
    pop
    pop - remove 1      :endone
    halt
";

    pub const FIBONACCI_VIT: &str = "
        push 0
        push 1
        push 7
        dup0
        dup0
        dup0
        mul
        eq
        skiz
        call bar
        call foo
   foo: call bob
        swap1
        push 18446744069414584320
        add
        dup0
        skiz
        recurse
        call baz
   bar: dup0
        push 0
        eq
        skiz
        pop
   baz: pop
        halt
   bob: dup2
        dup2
        add
        return
    ";

    pub const FIBONACCI_LT: &str = "
        push 0
        push 1
        push 7
        push 2
        dup1
        lt
        skiz
        call 29
        call 16
    16: call 38
        swap1
        push -1
        add
        dup0
        skiz
        recurse
        call 36
    29: dup0
        push 0
        eq
        skiz
        pop
    36: pop
        halt
    38: dup2
        dup2
        add
        return
    ";

    pub const GCD_X_Y: &str = "
           read_io
           read_io
           dup1
           dup1
           lt
           skiz
           swap1
loop_cond: dup0
           push 0
           eq
           skiz
           call terminate
           dup1
           dup1
           div
           swap2
           swap3
           pop
           pop
           call loop_cond
terminate: pop
           write_io
           halt
    ";

    // This cannot not print because we need to itoa() before write_io.
    // TODO: Swap0-7 are now available and we can continue this implementation.
    pub const XGCD: &str = "
        push 1
        push 0
        push 0
        push 1
        push 240
        push 46
    12: dup1
        dup1
        lt
        skiz
        swap1
        dup0
        push 0
        eq
        skiz
        call 33
        dup1
        dup1
        div
    33: swap2
        swap3
        pop
        pop
        call 12
        pop
        halt
    ";

    pub const HASH_HASH_HASH_HALT: &str = "
        hash
        hash
        hash
        halt
    ";

    pub const ALL_INSTRUCTIONS: &str = "
        pop
        push 42
        divine

        dup0 dup1 dup2 dup3 dup4 dup5 dup6 dup7 dup8 dup9 dup10 dup11 dup12 dup13 dup14 dup15
        swap1 swap2 swap3 swap4 swap5 swap6 swap7 swap8 swap9 swap10 swap11 swap12 swap13 swap14 swap15

        nop
        skiz
        call foo

        return recurse assert halt read_mem write_mem hash divine_sibling assert_vector
        add mul invert split eq lt and xor reverse div xxadd xxmul xinvert xbmul

        read_io write_io
    ";

    pub fn all_instructions_displayed() -> Vec<String> {
        vec![
            "pop",
            "push 42",
            "divine",
            "dup0",
            "dup1",
            "dup2",
            "dup3",
            "dup4",
            "dup5",
            "dup6",
            "dup7",
            "dup8",
            "dup9",
            "dup10",
            "dup11",
            "dup12",
            "dup13",
            "dup14",
            "dup15",
            "swap1",
            "swap2",
            "swap3",
            "swap4",
            "swap5",
            "swap6",
            "swap7",
            "swap8",
            "swap9",
            "swap10",
            "swap11",
            "swap12",
            "swap13",
            "swap14",
            "swap15",
            "nop",
            "skiz",
            "call foo",
            "return",
            "recurse",
            "assert",
            "halt",
            "read_mem",
            "write_mem",
            "hash",
            "divine_sibling",
            "assert_vector",
            "add",
            "mul",
            "invert",
            "split",
            "eq",
            "lt",
            "and",
            "xor",
            "reverse",
            "div",
            "xxadd",
            "xxmul",
            "xinvert",
            "xbmul",
            "read_io",
            "write_io",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

#[cfg(test)]
mod instruction_tests {
    use num_traits::{One, Zero};

    use super::{all_instructions_without_args, parse, sample_programs};
    use crate::instruction::all_labelled_instructions_with_args;
    use crate::ord_n::Ord7;
    use crate::vm::Program;

    #[test]
    fn parse_display_push_pop_test() {
        let pgm_expected = sample_programs::push_push_add_pop_p();
        let pgm_pretty = format!("{}", pgm_expected);
        let instructions = parse(&pgm_pretty).unwrap();
        let pgm_actual = Program::new(&instructions);

        assert_eq!(pgm_expected, pgm_actual);

        let pgm_text = sample_programs::PUSH_PUSH_ADD_POP_S;
        let instructions_2 = parse(&pgm_text).unwrap();
        let pgm_actual_2 = Program::new(&instructions_2);

        assert_eq!(pgm_expected, pgm_actual_2);
    }

    #[test]
    fn parse_and_display_each_instruction_test() {
        let expected = all_labelled_instructions_with_args();
        let actual = parse(sample_programs::ALL_INSTRUCTIONS).unwrap();

        assert_eq!(expected, actual);

        for (actual, expected) in actual
            .iter()
            .map(|instr| format!("{}", instr))
            .zip(sample_programs::all_instructions_displayed())
        {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn ib_registers_are_binary_test() {
        use Ord7::*;

        for instruction in all_instructions_without_args() {
            for ib in [IB0, IB1, IB2, IB3, IB4, IB5] {
                let ib_value = instruction.ib(ib);
                assert!(
                    ib_value.is_zero() || ib_value.is_one(),
                    "IB{} for instruction {} is 0 or 1 ({})",
                    ib,
                    instruction,
                    ib_value
                );
            }
        }
    }

    #[test]
    fn instruction_to_opcode_to_instruction_is_consistent_test() {
        for instr in all_instructions_without_args() {
            assert_eq!(instr, instr.opcode().try_into().unwrap());
        }
    }
}
