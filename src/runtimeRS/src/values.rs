#[repr(i64)]
#[derive(Debug, Clone, PartialEq)]
pub enum ToyType {
    Str = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    StrArr = 4,
    BoolArr = 5,
    IntArr = 6,
    FloatArr = 7,
    Struct = 8,
}
impl TryFrom<i64> for ToyType {
    type Error = i64;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ToyType::Str),
            1 => Ok(ToyType::Bool),
            2 => Ok(ToyType::Int),
            3 => Ok(ToyType::Float),
            4 => Ok(ToyType::StrArr),
            5 => Ok(ToyType::BoolArr),
            6 => Ok(ToyType::IntArr),
            7 => Ok(ToyType::FloatArr),
            8 => Ok(ToyType::Struct),
            _ => unreachable!()
        }
    }
}
impl From<ToyType> for i64 {
    fn from(t: ToyType) -> i64 { t as i64 }
}
impl ToyType {
    pub fn to_elem_type(&self) -> ToyType {
        return match self {
            &ToyType::StrArr => ToyType::Str,
            &ToyType::BoolArr => ToyType::Bool,
            &ToyType::IntArr => ToyType::Int,
            &ToyType::FloatArr => ToyType::Float,
            _ => panic!("[ERROR] Type {:?} does not have elements", self)
        };
    }
    pub fn to_arr_type(&self) -> ToyType {
        return match self {
            &ToyType::Str => ToyType::StrArr,
            &ToyType::Bool => ToyType::BoolArr,
            &ToyType::Int => ToyType::IntArr,
            &ToyType::Float => ToyType::FloatArr,
            _ => self.clone() //array type of int[] is int[]
        };
    }
    pub fn is_arr_type(&self) -> bool {
        return matches!(self, ToyType::StrArr | ToyType::BoolArr | ToyType::IntArr | ToyType::FloatArr);
    }
}