use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Program {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<Function>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Int,
    Bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Function {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Arg>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<Type>,
    #[serde(default)]
    pub instrs: Vec<Instruction>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Literal {
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectOp {
    Jmp,
    Br,
    Call,
    Ret,
    Print,
    Nop,
    Set,
    Speculate,
    Commit,
    Guard,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueOp {
    Add,
    Mul,
    Sub,
    Div,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
    Not,
    And,
    Or,
    Call,
    Id,
    Get,
    Undef,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstOps {
    Const,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Instruction {
    Constant {
        op: ConstOps,
        dest: String,
        #[serde(rename = "type")]
        ty: Type,
        value: Literal,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        span: Option<Span>,
    },
    Value {
        op: ValueOp,
        dest: String,
        #[serde(rename = "type")]
        ty: Type,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        funcs: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        labels: Vec<String>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        span: Option<Span>,
    },
    Effect {
        op: EffectOp,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        funcs: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        labels: Vec<String>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        span: Option<Span>,
    },
    Label {
        label: String,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        span: Option<Span>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Span {
    pub pos: Pos,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_end: Option<Pos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
}
