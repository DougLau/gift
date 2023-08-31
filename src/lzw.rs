// lzw.rs
//
// Copyright (c) 2020-2023  Douglas Lau
//
//! Lempel-Ziv-Welch compression for GIF
use crate::error::{Error, Result};
use std::cmp::Ordering;
use std::ops::AddAssign;

/// Code Bits
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bits(u8);

impl From<u8> for Bits {
    fn from(bits: u8) -> Self {
        Bits(bits.min(Self::MAX.0))
    }
}

impl From<Bits> for u8 {
    fn from(bits: Bits) -> Self {
        bits.0
    }
}

impl AddAssign<u8> for Bits {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = (self.0 + rhs).min(Self::MAX.0)
    }
}

impl Bits {
    /// Maximum code bits allowed for GIF
    const MAX: Self = Bits(12);

    /// Get the number of entries
    fn entries(self) -> u16 {
        1 << (self.0 as u16)
    }

    /// Get the bit mask
    fn mask(self) -> u32 {
        (1 << (self.0 as u32)) - 1
    }
}

/// Code type
type Code = u16;

/// Node for compressor table
#[derive(Clone, Copy, Debug)]
struct CNode {
    /// Next node code
    next: Option<Code>,
    /// Left node code
    left: Option<Code>,
    /// Right node code
    right: Option<Code>,
    /// Data value
    data: u8,
}

/// Compressor code table
#[derive(Debug)]
struct CTable(Vec<CNode>);

/// LZW Data Compressor
pub struct Compressor {
    /// Code table
    table: CTable,
    /// Minimum code bits
    min_code_bits: u8,
    /// Current code bits
    code_bits: Bits,
    /// Current code
    code: u32,
    /// Number of bits in current code
    n_bits: u8,
}

/// Node for decompressor table
#[derive(Clone, Copy, Debug)]
struct DNode {
    /// Parent node code
    parent: Option<Code>,
    /// Data value
    data: u8,
}

/// Decompressor code table
#[derive(Debug)]
struct DTable(Vec<DNode>);

/// LZW Data Decompressor
#[derive(Debug)]
pub struct Decompressor {
    /// Code table
    table: DTable,
    /// Minimum code bits
    min_code_bits: u8,
    /// Current code bits
    code_bits: Bits,
    /// Last code
    last: Option<Code>,
    /// Current code
    code: u32,
    /// Number of bits in current code
    n_bits: u8,
}

impl CNode {
    /// Create a new compressor node
    fn new(next: Option<Code>, data: u8) -> Self {
        CNode {
            next,
            left: None,
            right: None,
            data,
        }
    }

    /// Get a link code
    fn link(&self, ordering: Ordering) -> Option<Code> {
        match ordering {
            Ordering::Less => self.left,
            Ordering::Equal => self.next,
            Ordering::Greater => self.right,
        }
    }

    /// Set a link code
    fn set_link(&mut self, ordering: Ordering, code: Code) {
        match ordering {
            Ordering::Less => self.left = Some(code),
            Ordering::Equal => self.next = Some(code),
            Ordering::Greater => self.right = Some(code),
        }
    }
}

impl CTable {
    /// Create a new code table
    fn new(min_code_bits: u8) -> Self {
        let clear_code = 1 << min_code_bits;
        let mut table = CTable(Vec::with_capacity(Bits::MAX.entries().into()));
        table.reset(clear_code);
        table
    }

    /// Get the next available code
    fn next_code(&self) -> Code {
        self.0.len() as Code
    }

    /// Reset the table
    fn reset(&mut self, clear_code: Code) {
        self.0.clear();
        for data in 0..clear_code {
            self.push_node(None, data as u8);
        }
        self.push_node(None, 0); // clear code
        self.push_node(None, 0); // end code
    }

    /// Push a node into the table
    fn push_node(&mut self, next: Option<Code>, data: u8) {
        self.0.push(CNode::new(next, data))
    }

    /// Get a mutable node
    fn node_mut(&mut self, code: Code) -> &mut CNode {
        &mut self.0[code as usize]
    }

    /// Insert a node
    fn insert(&mut self, code: Code, data: u8) -> Option<Code> {
        let next_code = self.next_code();
        let mut node = self.node_mut(code);
        let mut ordering = Ordering::Equal;
        while let Some(code) = node.link(ordering) {
            node = self.node_mut(code);
            ordering = data.cmp(&node.data);
            if ordering == Ordering::Equal {
                return Some(code);
            }
        }
        node.set_link(ordering, next_code);
        self.push_node(None, data);
        None
    }

    /// Search and insert a node
    fn search_insert(&mut self, code: Option<Code>, data: u8) -> Option<Code> {
        match code {
            Some(code) => self.insert(code, data),
            None => Some(data as Code),
        }
    }
}

impl Compressor {
    /// Create a new compressor
    pub fn new(min_code_bits: u8) -> Self {
        let table = CTable::new(min_code_bits);
        let initial_code_bits = min_code_bits + 1;
        let code_bits = Bits::from(initial_code_bits);
        Compressor {
            table,
            min_code_bits,
            code_bits,
            code: 0,
            n_bits: 0,
        }
    }

    /// Get the clear code
    fn clear_code(&self) -> Code {
        1 << self.min_code_bits
    }

    /// Get the end code
    fn end_code(&self) -> Code {
        self.clear_code() + 1
    }

    /// Pack a code into a buffer
    fn pack(&mut self, code: Code, buffer: &mut Vec<u8>) {
        self.code |= (code as u32) << self.n_bits;
        self.n_bits += u8::from(self.code_bits);
        while self.n_bits >= 8 {
            buffer.push(self.code as u8);
            self.code >>= 8;
            self.n_bits -= 8;
        }
    }

    /// Compress a byte buffer
    pub fn compress(&mut self, bytes: &[u8], buffer: &mut Vec<u8>) {
        self.pack(self.clear_code(), buffer);
        let mut code = None;
        for data in bytes {
            code = self.table.search_insert(code, *data).or_else(|| {
                if let Some(code) = code {
                    self.pack(code, buffer);
                }
                Some(*data as Code)
            });
            let next_code = self.table.next_code();
            if next_code > self.code_bits.entries() {
                if next_code <= Bits::MAX.entries() {
                    self.code_bits += 1;
                } else {
                    self.pack(self.clear_code(), buffer);
                    self.table.reset(self.clear_code());
                    let initial_code_bits = self.min_code_bits + 1;
                    self.code_bits = Bits::from(initial_code_bits);
                }
            }
        }
        if let Some(code) = code {
            self.pack(code, buffer);
        }
        self.pack(self.end_code(), buffer);
    }
}

impl DTable {
    /// Create a new code table
    fn new(min_code_bits: u8) -> Self {
        let clear_code = 1 << min_code_bits;
        let mut table = DTable(Vec::with_capacity(Bits::MAX.entries().into()));
        table.reset(clear_code);
        table
    }

    /// Reset the table
    fn reset(&mut self, clear_code: Code) {
        self.0.clear();
        for data in 0..clear_code {
            self.push_node(None, data as u8);
        }
        self.push_node(None, 0); // clear code
        self.push_node(None, 0); // end code
    }

    /// Get the next available code
    fn next_code(&self) -> Code {
        self.0.len() as Code
    }

    /// Push a node into the table
    fn push_node(&mut self, parent: Option<Code>, data: u8) {
        self.0.push(DNode { parent, data });
    }

    /// Lookup data value of a code
    fn lookup(&self, code: Code) -> u8 {
        let mut node = self.0[code as usize];
        while let Some(code) = node.parent {
            node = self.0[code as usize];
        }
        node.data
    }

    /// Decompress a code into a buffer (reversed)
    fn decompress_reversed(&self, code: Code, buffer: &mut Vec<u8>) {
        let mut node = self.0[code as usize];
        while let Some(code) = node.parent {
            buffer.push(node.data);
            node = self.0[code as usize];
        }
        buffer.push(node.data);
    }
}

impl Decompressor {
    /// Create a new decompressr
    pub fn new(min_code_bits: u8) -> Self {
        let table = DTable::new(min_code_bits);
        let initial_code_bits = min_code_bits + 1;
        let code_bits = Bits::from(initial_code_bits);
        Decompressor {
            table,
            min_code_bits,
            code_bits,
            last: None,
            code: 0,
            n_bits: 0,
        }
    }

    /// Get the clear code
    fn clear_code(&self) -> Code {
        1 << self.min_code_bits
    }

    /// Get the end code
    fn end_code(&self) -> Code {
        self.clear_code() + 1
    }

    /// Unpack one code from a buffer
    fn unpack(&mut self, buffer: &[u8]) -> (Option<Code>, usize) {
        let mut n_consumed = 0;
        let code_bits = u8::from(self.code_bits);
        for data in buffer {
            if self.n_bits >= code_bits {
                break;
            }
            self.code |= (*data as u32) << self.n_bits;
            self.n_bits += 8;
            n_consumed += 1;
        }
        if self.n_bits >= code_bits {
            let code = (self.code & self.code_bits.mask()) as Code;
            self.code >>= code_bits;
            self.n_bits -= code_bits;
            (Some(code), n_consumed)
        } else {
            (None, n_consumed)
        }
    }

    /// Decompress a byte buffer
    pub fn decompress(
        &mut self,
        bytes: &[u8],
        buffer: &mut Vec<u8>,
    ) -> Result<()> {
        let mut bytes = bytes;
        while let (Some(code), n_consumed) = self.unpack(bytes) {
            self.decompress_code(code, buffer)?;
            bytes = &bytes[n_consumed..];
        }
        Ok(())
    }

    /// Decompress one code
    fn decompress_code(
        &mut self,
        code: Code,
        buffer: &mut Vec<u8>,
    ) -> Result<()> {
        if code == self.clear_code() {
            self.table.reset(self.clear_code());
            let initial_code_bits = self.min_code_bits + 1;
            self.code_bits = Bits::from(initial_code_bits);
            self.last = None;
        } else if code != self.end_code() {
            let start = buffer.len();
            self.decompress_reversed(code, buffer)?;
            buffer[start..].reverse();
            self.last = Some(code);
        }
        Ok(())
    }

    /// Decompress one code (reversed)
    fn decompress_reversed(
        &mut self,
        code: Code,
        buffer: &mut Vec<u8>,
    ) -> Result<()> {
        let next_code = self.table.next_code();
        match (self.last, code.cmp(&next_code)) {
            (_, Ordering::Greater) => return Err(Error::InvalidLzwData),
            (Some(last), Ordering::Less) => {
                self.table.decompress_reversed(code, buffer);
                let data = buffer.last().copied().unwrap();
                self.table.push_node(Some(last), data);
            }
            (Some(last), Ordering::Equal) => {
                self.table.push_node(Some(last), self.table.lookup(last));
                self.table.decompress_reversed(code, buffer);
            }
            (None, _) => buffer.push(code as u8),
        }
        if next_code + 1 == self.code_bits.entries() {
            self.code_bits += 1;
        }
        Ok(())
    }
}
