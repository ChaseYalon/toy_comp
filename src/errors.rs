use crate::token::Token;
use colored::*;
use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;
use std::backtrace::Backtrace;
use std::fmt::*;
use std::{fmt, fs};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Span {
    pub file_path: String,
    ///number of bytes before the code in question is started. INCLUSIVE
    pub start_offset_bytes: i64,
    ///number of bytes FROM THE BEGINNING that marks the end of the span, INCLUSIVE
    pub end_offset_bytes: i64,
}
impl Span {
    ///end_offset_bytes - number of bytes before the code in question is started. INCLUSIVE
    ///end_offset_bytes - number of bytes FROM THE BEGINNING that marks the end of the span, INCLUSIVE
    pub fn new(path: &str, start_offset_bytes: i64, end_offset_bytes: i64) -> Span {
        return Span {
            file_path: path.to_string(),
            start_offset_bytes,
            end_offset_bytes,
        };
    }
    pub fn null_span() -> Span{
        return Span::new("NULL", -1, -1);
    }
    pub fn null_span_with_msg(msg: &str) -> Span{
        return Span::new(msg, -1, -1);
    }

    ///SAFETY - this is considered an internal compiler function, any error in here is a compiler error that should crash
    pub fn to_string(&self) -> String {
        return String::from_utf8(
            fs::read_to_string(self.file_path.clone())
                .unwrap()
                .as_bytes()
                .to_vec()[self.start_offset_bytes as usize..=self.end_offset_bytes as usize]
                .to_vec(),
        )
        .unwrap();
    }
    //(line, col), (line, col)
    pub fn get_line_col(&self) -> ((u64, u64), (u64, u64)) {
        let content = fs::read_to_string(self.file_path.clone()).unwrap();
        let mut line = 1u64;
        let mut col = 1u64;
        let mut start_line = 1u64;
        let mut start_col = 1u64;
        let mut end_line = 1u64;
        let mut end_col = 1u64;

        for (byte_idx, ch) in content.bytes().enumerate() {
            let idx = byte_idx as i64;

            if idx == self.start_offset_bytes {
                start_line = line;
                start_col = col;
            }

            if idx == self.end_offset_bytes {
                end_line = line;
                end_col = col;
            }

            if ch == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        ((start_line, start_col), (end_line, end_col))
    }
}
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
    IncorrectNumberOfArguments,
}

#[derive(Debug, Error)]
pub struct ToyError {
    error_type: ToyErrorType,
    backtrace: Backtrace,
    offending_code: Span,
}

impl ToyError {
    pub fn new(i_error_type: ToyErrorType, offending_code: Span) -> ToyError {
        return ToyError {
            error_type: i_error_type,
            backtrace: Backtrace::capture(),
            offending_code: if offending_code.end_offset_bytes != -1 {
                offending_code
            } else {
                Span::new("FILE_NOT_SPECIFIED",-1, -1)
            },
        };
    }

    pub fn with_context(mut self, context: Span) -> Self {
        if self.offending_code.end_offset_bytes == -1 {
            self.offending_code = context;
        }
        self
    }
}

impl fmt::Display for ToyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offending_code.end_offset_bytes > 0{
            let ((start_l, start_c), (end_l, end_c)) = self.offending_code.get_line_col();
            write!(
                f,
                "\n{}\nType: {}\nProblematic Code: {}\nLine Number: {}_{} : {}_{}\nBacktrace:\n{}",
                "[ERROR]".red().bold(),
                self.error_type.to_string().blue().bold(),
                self.offending_code.to_string().magenta(),
                start_l.to_string().green(),
                start_c.to_string().green(),
                end_l.to_string().green(),
                end_c.to_string().green(),
                self.backtrace.to_string()
            )
        } else {
            write!(
                f,
                "\n{}\nType: {}\nError hint: {}\nBacktrace:\n{}",
                "[ERROR]".red().bold(),
                self.error_type.to_string().blue().bold(),
                self.offending_code.to_string().magenta(),
                self.backtrace.to_string()
            )
        }
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
            Self::InvalidOperationOnGivenType => write!(f, "Invalid Operation On Given Type"),
            Self::MissingFile => write!(f, "Missing File"),
            Self::VariableNotAStruct => write!(f, "VariableNotAStruct"),
            _ => todo!("chase implement error type {:?}", self),
        }
    }
}

impl From<BuilderError> for ToyError {
    fn from(err: BuilderError) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()), Span::null_span());
    }
}
impl From<LLVMString> for ToyError {
    fn from(err: LLVMString) -> Self {
        return ToyError::new(ToyErrorType::LlvmError(err.to_string()), Span::null_span());
    }
}
