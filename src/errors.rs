use crate::token::Token;
use colored::*;
use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;
use std::backtrace::Backtrace;
use std::fmt;
use std::fmt::*;
use thiserror::Error;
#[derive(Debug)]
pub enum ToyErrorType {
    InternalFunctionUndefined,
    InternalLinkerFailure,
    InternalParserFailure,
    InvalidInfixOperation,
    ExpectedToken(Token), //missing '('
    ExpectedIdentifier,
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
    MalformedType,
    MalformedFuncCall,
    ExpressionNotNumeric,
    MissingInstruction,
    LlvmError(String),
    UndefinedSSAValue,
    MalformedImportStatement,
    MissingFile,
    IncorrectNumberOfArguments
}

#[derive(Debug, Error)]
pub struct ToyError {
    error_type: ToyErrorType,
    backtrace: Backtrace,
    offending_code: String,
}

impl ToyError {
    pub fn new(i_error_type: ToyErrorType, offending_code: Option<String>) -> ToyError {
        return ToyError {
            error_type: i_error_type,
            backtrace: Backtrace::capture(),
            offending_code: if offending_code.is_some() {
                offending_code.unwrap()
            } else {
                "code segment unknown".to_string()
            },
        };
    }
}

impl fmt::Display for ToyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n{}\nType: {}\nProblematic Code: {}\nBacktrace:\n{}",
            "[ERROR]".red().bold(),
            self.error_type.to_string().blue().bold(),
            self.offending_code.magenta(),
            self.backtrace.to_string()
        )
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
            Self::MalformedType => write!(f, "Malformed Type"),
            Self::MalformedFuncCall => write!(f, "Malformed FuncCall"),
            Self::TypeHintNeeded => write!(f, "TypeHintNeeded"),
            Self::MissingInstruction => write!(f, "MissingInstruction"),
            Self::LlvmError(s) => write!(f, "Llvm Error ({})", s),
            Self::UndefinedFunction => write!(f, "Undefined Function"),
            Self::UndefinedInterface => write!(f, "Undefined Interface"),
            Self::UndefinedStruct => write!(f, "Undefined Struct"),
            Self::UndefinedUnresolvedStruct => write!(f, "Undefined Unresolved Struct"),
            Self::UnsupportedOS => write!(f, "Unsupported OS"),
            Self::ExpressionNotNumeric => write!(f, "Expression Not Numeric"),
            Self::ExpressionNotBoolean => write!(f, "Expression Not Boolean"),
            Self::ArrayTypeInvalid => write!(f, "Array Type Invalid"),
            Self::KeyNotOnStruct => write!(f, "Key Not On Struct"),
            Self::TypeMismatch => write!(f, "Type Mismatch"),
            Self::MissingFile => write!(f, "Missing File"),
            _ => todo!("chase implement error type {:?}", self),
        }
    }
}

impl From<BuilderError> for ToyError {
    fn from(err: BuilderError) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()), None);
    }
}
impl From<LLVMString> for ToyError {
    fn from(err: LLVMString) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()), None);
    }
}
