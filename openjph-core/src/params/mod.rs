//! JPEG 2000 codestream parameter marker segments (SIZ, COD, QCD, etc.)
//!
//! Port of `ojph_params.h`, `ojph_params_local.h`, and `ojph_params.cpp`.

pub(crate) mod local;

// Re-export public types
pub use local::{
    ParamSiz, ParamCod, ParamQcd, ParamCap, ParamSot, ParamTlm,
    ParamNlt, ParamDfs, CommentExchange, TtlmPtlmPair,
    ProgressionOrder, ProfileNum,
};
