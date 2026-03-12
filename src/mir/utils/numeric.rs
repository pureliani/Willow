use crate::mir::types::checked_type::Type;

pub fn get_numeric_type_rank(ty: &Type) -> Option<i32> {
    use Type::*;
    match &ty {
        I8(_) | U8(_) => Some(1),
        I16(_) | U16(_) => Some(2),
        I32(_) | U32(_) | ISize(_) | USize(_) => Some(3),
        I64(_) | U64(_) => Some(4),
        F32(_) => Some(5),
        F64(_) => Some(6),
        _ => None,
    }
}

pub fn is_float(ty: &Type) -> bool {
    use Type::*;
    matches!(ty, F32(_) | F64(_))
}

pub fn is_integer(ty: &Type) -> bool {
    use Type::*;
    matches!(
        ty,
        I8(_)
            | I16(_)
            | I32(_)
            | I64(_)
            | U8(_)
            | U16(_)
            | U32(_)
            | U64(_)
            | ISize(_)
            | USize(_)
    )
}

pub fn is_signed(ty: &Type) -> bool {
    use Type::*;
    matches!(
        ty,
        I8(_) | I16(_) | I32(_) | I64(_) | ISize(_) | F32(_) | F64(_)
    )
}
