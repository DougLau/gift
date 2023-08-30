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

/// Node for code dictionary
trait Node {
    /// Create a new node
    fn new(next: Option<Code>, byte: u8) -> Self;

    /// Get the next node
    fn next(self) -> Option<Code>;

    /// Get the byte value
    fn byte(self) -> u8;
}

/// Node for Compressor
#[derive(Clone, Copy, Debug)]
struct CNode {
    /// Next node code
    next: Option<Code>,
    /// Left node code
    left: Option<Code>,
    /// Right node code
    right: Option<Code>,
    /// Byte value
    byte: u8,
}

/// Node for Decompressor
#[derive(Clone, Copy, Debug)]
struct DNode {
    /// Next node code
    next: Option<Code>,
    /// Byte value
    byte: u8,
}

/// Code dictionary trie
#[derive(Debug)]
struct Trie<N: Node> {
    /// Table of codes
    table: Vec<N>,
    /// Clear code
    clear_code: Code,
}

/// LZW Data Compressor
pub struct Compressor {
    /// Code dictionary
    trie: Trie<CNode>,
    /// Initial code bits
    initial_code_bits: u8,
    /// Current code bits
    code_bits: Bits,
    /// Current code
    code: u32,
    /// Number of bits in current code
    n_bits: u8,
}

/// LZW Data Decompressor
#[derive(Debug)]
pub struct Decompressor {
    /// Code dictionary
    trie: Trie<DNode>,
    /// Initial code bits
    initial_code_bits: u8,
    /// Current code bits
    code_bits: Bits,
    /// Last code
    last: Option<Code>,
    /// Current code
    code: u32,
    /// Number of bits in current code
    n_bits: u8,
}

impl Node for CNode {
    fn new(next: Option<Code>, byte: u8) -> Self {
        CNode {
            next,
            left: None,
            right: None,
            byte,
        }
    }

    fn next(self) -> Option<Code> {
        self.next
    }

    fn byte(self) -> u8 {
        self.byte
    }
}

impl Node for DNode {
    fn new(next: Option<Code>, byte: u8) -> Self {
        DNode { next, byte }
    }

    fn next(self) -> Option<Code> {
        self.next
    }

    fn byte(self) -> u8 {
        self.byte
    }
}

impl CNode {
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

impl<N: Node> Trie<N> {
    /// Create a new code dictionary
    fn new(min_code_bits: u8) -> Self {
        let clear_code = 1 << min_code_bits;
        let mut table = Vec::with_capacity(Bits::MAX.entries().into());
        for byte in 0..clear_code {
            table.push(N::new(None, byte as u8));
        }
        table.push(N::new(None, 0)); // clear code
        table.push(N::new(None, 0)); // end code
        Trie {
            table,
            clear_code,
        }
    }

    /// Get the clear code
    fn clear_code(&self) -> Code {
        self.clear_code
    }

    /// Get the end code
    fn end_code(&self) -> Code {
        self.clear_code() + 1
    }

    /// Get the next available code
    fn next_code(&self) -> Code {
        self.table.len() as Code
    }

    /// Reset the dictionary
    fn reset(&mut self) {
        let len = usize::from(self.end_code()) + 1;
        self.table.truncate(len);
    }

    /// Push a node into the dictionary
    fn push_node(&mut self, next: Option<Code>, byte: u8) {
        self.table.push(N::new(next, byte))
    }
}

impl Trie<CNode> {
    /// Get a mutable node
    fn node_mut(&mut self, code: Code) -> &mut CNode {
        &mut self.table[code as usize]
    }

    /// Insert a node
    fn insert(&mut self, code: Code, byte: u8) -> Option<Code> {
        let next_code = self.next_code();
        let mut node = self.node_mut(code);
        let mut ordering = Ordering::Equal;
        while let Some(code) = node.link(ordering) {
            node = self.node_mut(code);
            ordering = byte.cmp(&node.byte());
            if ordering == Ordering::Equal {
                return Some(code);
            }
        }
        node.set_link(ordering, next_code);
        self.push_node(None, byte);
        None
    }

    /// Search and insert a node
    fn search_insert(&mut self, code: Option<Code>, byte: u8) -> Option<Code> {
        match code {
            Some(code) => self.insert(code, byte),
            None => Some(byte as Code),
        }
    }
}

impl Compressor {
    /// Create a new compressor
    pub fn new(min_code_bits: u8) -> Self {
        let trie = Trie::<CNode>::new(min_code_bits);
        let initial_code_bits = min_code_bits + 1;
        let code_bits = Bits::from(initial_code_bits);
        Compressor {
            initial_code_bits,
            trie,
            code_bits,
            code: 0,
            n_bits: 0,
        }
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
        self.pack(self.trie.clear_code(), buffer);
        let mut code = None;
        for byte in bytes {
            code = self.trie.search_insert(code, *byte).or_else(|| {
                if let Some(code) = code {
                    self.pack(code, buffer);
                }
                Some(*byte as Code)
            });
            let next_code = self.trie.next_code();
            if next_code > self.code_bits.entries() {
                if next_code <= Bits::MAX.entries() {
                    self.code_bits += 1;
                } else {
                    self.pack(self.trie.clear_code(), buffer);
                    self.trie.reset();
                    self.code_bits = Bits::from(self.initial_code_bits);
                }
            }
        }
        if let Some(code) = code {
            self.pack(code, buffer);
        }
        self.pack(self.trie.end_code(), buffer);
    }
}

impl Trie<DNode> {
    /// Lookup a code value
    fn lookup(&self, code: Code) -> u8 {
        let mut node = self.table[code as usize];
        while let Some(code) = node.next {
            node = self.table[code as usize];
        }
        node.byte()
    }

    /// Decompress a code into a buffer (reversed)
    fn decompress_reversed(&self, code: Code, buffer: &mut Vec<u8>) {
        let mut node = self.table[code as usize];
        while let Some(code) = node.next {
            buffer.push(node.byte());
            node = self.table[code as usize];
        }
        buffer.push(node.byte());
    }
}

impl Decompressor {
    /// Create a new decompressr
    pub fn new(min_code_bits: u8) -> Self {
        let trie = Trie::<DNode>::new(min_code_bits);
        let initial_code_bits = min_code_bits + 1;
        let code_bits = Bits::from(initial_code_bits);
        Decompressor {
            initial_code_bits,
            trie,
            code_bits,
            last: None,
            code: 0,
            n_bits: 0,
        }
    }

    /// Unpack one code from a buffer
    fn unpack(&mut self, buffer: &[u8]) -> (Option<Code>, usize) {
        let mut n_consumed = 0;
        let code_bits = u8::from(self.code_bits);
        for byte in buffer {
            if self.n_bits >= code_bits {
                break;
            }
            self.code |= (*byte as u32) << self.n_bits;
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
        if code == self.trie.clear_code() {
            self.trie.reset();
            self.code_bits = Bits::from(self.initial_code_bits);
            self.last = None;
        } else if code != self.trie.end_code() {
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
        let next_code = self.trie.next_code();
        match (self.last, code.cmp(&next_code)) {
            (_, Ordering::Greater) => return Err(Error::InvalidLzwData),
            (Some(last), Ordering::Less) => {
                self.trie.decompress_reversed(code, buffer);
                let byte = buffer.last().copied().unwrap();
                self.trie.push_node(Some(last), byte);
            }
            (Some(last), Ordering::Equal) => {
                self.trie.push_node(Some(last), self.trie.lookup(last));
                self.trie.decompress_reversed(code, buffer);
            }
            (None, _) => buffer.push(code as u8),
        }
        if next_code + 1 == self.code_bits.entries() {
            self.code_bits += 1;
        }
        Ok(())
    }
}
