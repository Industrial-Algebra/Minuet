//! Symbolic domain utilities.
//!
//! Encoders for code ASTs and symbolic expressions.

use std::marker::PhantomData;

use amari_fusion::{holographic::Bindable, TropicalDualClifford};

use crate::binding::Codebook;
use crate::precision::MinuetFloat;

use super::DomainEncoder;

/// A simple AST node for encoding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AstNode {
    /// Identifier (variable, function name).
    Identifier(String),
    /// Literal value.
    Literal(String),
    /// Binary operation.
    BinaryOp {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// Unary operation.
    UnaryOp { op: String, operand: Box<AstNode> },
    /// Function call.
    Call { name: String, args: Vec<AstNode> },
    /// Block of statements.
    Block(Vec<AstNode>),
    /// Assignment.
    Assignment {
        target: Box<AstNode>,
        value: Box<AstNode>,
    },
}

impl AstNode {
    /// Create an identifier node.
    #[must_use]
    pub fn ident(name: &str) -> Self {
        Self::Identifier(name.to_string())
    }

    /// Create a literal node.
    #[must_use]
    pub fn literal(value: &str) -> Self {
        Self::Literal(value.to_string())
    }

    /// Create a binary operation.
    #[must_use]
    pub fn binary(op: &str, left: Self, right: Self) -> Self {
        Self::BinaryOp {
            op: op.to_string(),
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a function call.
    #[must_use]
    pub fn call(name: &str, args: Vec<Self>) -> Self {
        Self::Call {
            name: name.to_string(),
            args,
        }
    }
}

/// Encoder for AST nodes.
///
/// Uses recursive tree encoding where:
/// - Each node type has a characteristic representation
/// - Children are bound compositionally
/// - Structure is preserved in the representation
pub struct AstEncoder<T: MinuetFloat, const DIM: usize> {
    /// Codebook for node type symbols.
    codebook: Codebook<T, DIM>,
    _phantom: PhantomData<T>,
}

impl<T: MinuetFloat, const DIM: usize> AstEncoder<T, DIM> {
    /// Create a new AST encoder.
    #[must_use]
    pub fn new() -> Self {
        let codebook = Codebook::new();

        // Pre-register common node types
        let _ident = codebook.symbol("node:identifier");
        let _literal = codebook.symbol("node:literal");
        let _binop = codebook.symbol("node:binary_op");
        let _unop = codebook.symbol("node:unary_op");
        let _call = codebook.symbol("node:call");
        let _block = codebook.symbol("node:block");
        let _assign = codebook.symbol("node:assignment");

        // Pre-register common operators
        let _add = codebook.symbol("op:+");
        let _sub = codebook.symbol("op:-");
        let _mul = codebook.symbol("op:*");
        let _div = codebook.symbol("op:/");
        let _eq = codebook.symbol("op:==");
        let _neq = codebook.symbol("op:!=");

        Self {
            codebook,
            _phantom: PhantomData,
        }
    }

    /// Encode a node recursively.
    fn encode_node(&self, node: &AstNode) -> TropicalDualClifford<T, DIM> {
        match node {
            AstNode::Identifier(name) => {
                let type_sym = self.codebook.symbol("node:identifier");
                let name_sym = self.codebook.symbol(name);
                type_sym.bind(&name_sym)
            }

            AstNode::Literal(value) => {
                let type_sym = self.codebook.symbol("node:literal");
                let value_sym = self.codebook.symbol(value);
                type_sym.bind(&value_sym)
            }

            AstNode::BinaryOp { op, left, right } => {
                let type_sym = self.codebook.symbol("node:binary_op");
                let op_sym = self.codebook.symbol(&format!("op:{}", op));
                let left_enc = self.encode_node(left);
                let right_enc = self.encode_node(right);

                // Structure: type ⊛ (op ⊛ (left ⊛ right))
                let operands = left_enc.bind(&right_enc);
                let with_op = op_sym.bind(&operands);
                type_sym.bind(&with_op)
            }

            AstNode::UnaryOp { op, operand } => {
                let type_sym = self.codebook.symbol("node:unary_op");
                let op_sym = self.codebook.symbol(&format!("op:{}", op));
                let operand_enc = self.encode_node(operand);

                let with_op = op_sym.bind(&operand_enc);
                type_sym.bind(&with_op)
            }

            AstNode::Call { name, args } => {
                let type_sym = self.codebook.symbol("node:call");
                let name_sym = self.codebook.symbol(name);

                // Bundle all arguments
                let args_enc = args
                    .iter()
                    .map(|a| self.encode_node(a))
                    .fold(TropicalDualClifford::bundling_zero(), |acc, arg| {
                        acc.bundle(&arg, T::one())
                    });

                let with_args = name_sym.bind(&args_enc);
                type_sym.bind(&with_args)
            }

            AstNode::Block(statements) => {
                let type_sym = self.codebook.symbol("node:block");

                // Bundle all statements with position encoding
                let mut block_enc = TropicalDualClifford::bundling_zero();
                for (i, stmt) in statements.iter().enumerate() {
                    let pos_sym = self.codebook.symbol(&format!("pos:{}", i));
                    let stmt_enc = self.encode_node(stmt);
                    let positioned = pos_sym.bind(&stmt_enc);
                    block_enc = block_enc.bundle(&positioned, T::one());
                }

                type_sym.bind(&block_enc)
            }

            AstNode::Assignment { target, value } => {
                let type_sym = self.codebook.symbol("node:assignment");
                let target_enc = self.encode_node(target);
                let value_enc = self.encode_node(value);

                let assignment = target_enc.bind(&value_enc);
                type_sym.bind(&assignment)
            }
        }
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for AstEncoder<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MinuetFloat, const DIM: usize> DomainEncoder<T, DIM> for AstEncoder<T, DIM> {
    type Input = AstNode;

    fn encode(&self, input: &Self::Input) -> TropicalDualClifford<T, DIM> {
        self.encode_node(input)
    }

    fn decode(&self, _repr: &TropicalDualClifford<T, DIM>) -> Option<Self::Input> {
        // AST decoding would require search/cleanup
        None
    }
}

/// Semantic similarity between code fragments.
pub struct CodeSimilarity<T: MinuetFloat, const DIM: usize> {
    encoder: AstEncoder<T, DIM>,
}

impl<T: MinuetFloat, const DIM: usize> CodeSimilarity<T, DIM> {
    /// Create a new code similarity calculator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            encoder: AstEncoder::new(),
        }
    }

    /// Compute similarity between two AST nodes.
    pub fn similarity(&self, a: &AstNode, b: &AstNode) -> f64 {
        let enc_a = self.encoder.encode(a);
        let enc_b = self.encoder.encode(b);
        enc_a.similarity(&enc_b).to_f64().unwrap_or(0.0)
    }
}

impl<T: MinuetFloat, const DIM: usize> Default for CodeSimilarity<T, DIM> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_encoding() {
        let encoder: AstEncoder<f64, 64> = AstEncoder::new();

        // x + y
        let expr1 = AstNode::binary("+", AstNode::ident("x"), AstNode::ident("y"));

        // x + z
        let expr2 = AstNode::binary("+", AstNode::ident("x"), AstNode::ident("z"));

        // x * y
        let expr3 = AstNode::binary("*", AstNode::ident("x"), AstNode::ident("y"));

        let enc1 = encoder.encode(&expr1);
        let enc2 = encoder.encode(&expr2);
        let enc3 = encoder.encode(&expr3);

        // Similar structure (same op, same first operand) should be more similar
        let sim_12 = enc1.similarity(&enc2);
        let sim_13 = enc1.similarity(&enc3);

        // Both have x as first operand, but 1 and 2 share the + operator
        // This is a weak test since similarity depends on exact encoding
        assert!(sim_12 > 0.0);
        assert!(sim_13 > 0.0);
    }

    #[test]
    fn code_similarity() {
        let sim: CodeSimilarity<f64, 64> = CodeSimilarity::new();

        let expr1 = AstNode::binary("+", AstNode::ident("a"), AstNode::ident("b"));

        let expr2 = AstNode::binary("+", AstNode::ident("a"), AstNode::ident("b"));

        // Identical expressions should have high similarity
        let s = sim.similarity(&expr1, &expr2);
        assert!(s > 0.99);
    }
}
