use crate::token::Token;
use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;
use std::backtrace::Backtrace;
use std::fmt;
use thiserror::Error;
#[derive(Debug)]
pub enum ToyErrorType {
    InternalFunctionUndefined,
    InternalLinkerFailure,
    InternalParserFailure,
    InvalidInfixOperation,
    ExpectedToken(Token), //missing '('
    ExpectedName(Box<String>),
    ExpectedExpression,
    InvalidArrayReference,
    InvalidLocationForBreakStatement,
    InvalidLocationForContinueStatement,
    MalformedFunctionDeclaration,
    MalformedLetStatement,
    UnclosedDelimiter,
    MalformedStructField,
    MalformedVariableReassign,
    MalformedWhileStatement,
    MalformedFieldName,
    UnknownSymbol(Token),
    VariableNotAStruct,
    InvalidOperationOnGivenType,
    ArrayElementsMustMatchArrayType,
    ExpressionNotBoolean,
    ArrayTypeInvalid,
    KeyNotOnStruct,
    TypeMismatch,
    TypeHintNeeded,
    TypeIdNotAssigned,
    VariableOfWrongType,
    UndefinedFunction,
    UndefinedInterface,
    UndefinedStruct,
    UndefinedUnresolvedStruct,
    UnsupportedOS,
    UndefinedVariable,
    UnknownCharacter(char),
    MalformedStructInterface,
    MalformedFuncCall,
    ExpressionNotNumeric,
    MissingInstruction,
    LlvmError(String),
}

#[derive(Debug, Error)]
pub struct ToyError {
    error_type: ToyErrorType,
    backtrace: Backtrace,
}

impl fmt::Display for ToyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error: {}", self.error_type)?;
        writeln!(f, "Stack trace:")?;

        let backtrace_str = format!("{}", self.backtrace);

        // Filter out noise and format cleanly
        for (i, line) in backtrace_str.lines().enumerate() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with("Backtrace") {
                writeln!(f, "  {}: {}", i, line)?;
            }
        }

        Ok(())
    }
}
impl ToyError {
    pub fn new(i_error_type: ToyErrorType) -> ToyError {
        return ToyError {
            error_type: i_error_type,
            backtrace: Backtrace::capture(),
        };
    }
}

impl fmt::Display for ToyErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalFunctionUndefined => write!(f, "Internal Function Undefined"),
            Self::InternalLinkerFailure => write!(f, "Internal Linker Failure"),
            Self::InternalParserFailure => write!(f, "Internal Parser Failure"),
            Self::InvalidInfixOperation => write!(f, "Invalid Infix Operation"),
            Self::ExpectedToken(token) => write!(f, "Expected Token: {:?}", token),
            Self::ExpectedName(name) => write!(f, "Expected Name: {:?}", name),
            Self::ExpectedExpression => write!(f, "Expected Expression"),
            Self::InvalidArrayReference => write!(f, "Invalid Array Reference"),
            Self::InvalidLocationForBreakStatement => {
                write!(f, "Invalid Location For Break Statement")
            }
            Self::InvalidLocationForContinueStatement => {
                write!(f, "Invalid Location For Continue Statement")
            }
            Self::MalformedFunctionDeclaration => write!(f, "Malformed Function Declaration"),
            Self::MalformedLetStatement => write!(f, "Malformed Let"),
            Self::UnclosedDelimiter => write!(f, "Unclosed Delimiter"),
            Self::MalformedStructField => write!(f, "Malformed Struct Field"),
            Self::MalformedVariableReassign => write!(f, "Malformed Variable Reassign"),
            Self::MalformedWhileStatement => write!(f, "Malformed While Statement"),
            Self::MalformedFieldName => write!(f, "Malformed Field Name"),
            Self::UnknownSymbol(token) => write!(f, "Unknown Symbol: {:?}", token),
            Self::UndefinedVariable => write!(f, "Undefined Variable"),
            Self::UnknownCharacter(token) => write!(f, "Unknown Character: {:?}", token),
            Self::MalformedStructInterface => write!(f, "Malformed Struct"),
            Self::MalformedFuncCall => write!(f, "Malformed FuncCall"),
            Self::TypeHintNeeded => write!(f, "TypeHintNeeded"),
            Self::MissingInstruction => write!(f, "MissingInstruction"),
            Self::LlvmError(s) => write!(f, "Llvm Error ({})", s),
            _ => todo!("chase implement error type {:?}", self),
        }
    }
}

impl From<BuilderError> for ToyError {
    fn from(err: BuilderError) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()));
    }
}
impl From<LLVMString> for ToyError {
    fn from(err: LLVMString) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()));
    }
}
