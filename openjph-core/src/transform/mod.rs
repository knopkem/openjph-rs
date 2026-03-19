//! Wavelet and color transforms (DWT 5/3, 9/7, RCT, ICT).
//!
//! Port of `ojph_transform.h/cpp` and `ojph_colour.h/cpp`.

pub(crate) mod wavelet;
pub(crate) mod colour;
pub(crate) mod simd;

use std::sync::OnceLock;

use crate::mem::LineBuf;

// =========================================================================
// Lifting step — port of C++ `union lifting_step`
// =========================================================================

/// Reversible lifting step parameters (5/3 DWT).
#[derive(Debug, Clone, Copy, Default)]
pub struct RevLiftingStep {
    /// Lifting coefficient (Aatk).
    pub a: i16,
    /// Additive residue (Batk).
    pub b: i16,
    /// Power-of-2 shift (Eatk).
    pub e: u8,
}

/// Irreversible lifting step parameters (9/7 DWT).
#[derive(Debug, Clone, Copy, Default)]
pub struct IrvLiftingStep {
    /// Lifting coefficient (Aatk).
    pub a: f32,
}

/// A single lifting step — either reversible (integer) or irreversible (float).
#[derive(Debug, Clone, Copy)]
pub enum LiftingStep {
    Reversible(RevLiftingStep),
    Irreversible(IrvLiftingStep),
}

impl Default for LiftingStep {
    fn default() -> Self {
        LiftingStep::Reversible(RevLiftingStep::default())
    }
}

impl LiftingStep {
    /// Access as reversible step. Panics if irreversible.
    #[inline]
    pub fn rev(&self) -> &RevLiftingStep {
        match self {
            LiftingStep::Reversible(r) => r,
            _ => panic!("expected reversible lifting step"),
        }
    }

    /// Access as irreversible step. Panics if reversible.
    #[inline]
    pub fn irv(&self) -> &IrvLiftingStep {
        match self {
            LiftingStep::Irreversible(i) => i,
            _ => panic!("expected irreversible lifting step"),
        }
    }
}

// =========================================================================
// ParamAtk — port of C++ `struct param_atk`
// =========================================================================

/// Maximum number of inline lifting steps (matches C++ d_store[6]).
const MAX_INLINE_STEPS: usize = 6;

/// Arbitrary Transformation Kernel parameters.
///
/// Stores the lifting steps for one wavelet kernel (e.g., the standard 5/3 or
/// 9/7 filter).
#[derive(Debug, Clone)]
pub struct ParamAtk {
    /// ATK marker segment length.
    pub latk: u16,
    /// Satk — carries filter type information.
    pub satk: u16,
    /// Scaling factor K (irreversible only).
    pub katk: f32,
    /// Number of lifting steps.
    pub natk: u8,
    /// The lifting step coefficients.
    pub steps: Vec<LiftingStep>,
}

impl Default for ParamAtk {
    fn default() -> Self {
        Self {
            latk: 0,
            satk: 0,
            katk: 0.0,
            natk: 0,
            steps: Vec::with_capacity(MAX_INLINE_STEPS),
        }
    }
}

impl ParamAtk {
    /// Returns the number of lifting steps.
    #[inline]
    pub fn get_num_steps(&self) -> u32 {
        self.natk as u32
    }

    /// Returns a reference to the `s`-th lifting step.
    #[inline]
    pub fn get_step(&self, s: u32) -> &LiftingStep {
        debug_assert!((s as u8) < self.natk);
        &self.steps[s as usize]
    }

    /// Returns the scaling factor K (irreversible kernels).
    #[inline]
    pub fn get_k(&self) -> f32 {
        self.katk
    }

    /// Initializes for the standard irreversible 9/7 wavelet.
    pub fn init_irv97(&mut self) {
        // 9/7 lifting coefficients (4 steps)
        const ALPHA: f32 = -1.586_134_3; // step 0
        const BETA: f32 = -0.052_980_118; // step 1
        const GAMMA: f32 = 0.882_911_08; // step 2
        const DELTA: f32 = 0.443_506_85; // step 3
        const K: f32 = 1.230_174_1;

        self.natk = 4;
        self.katk = K;
        self.steps.clear();
        self.steps.push(LiftingStep::Irreversible(IrvLiftingStep { a: ALPHA }));
        self.steps.push(LiftingStep::Irreversible(IrvLiftingStep { a: BETA }));
        self.steps.push(LiftingStep::Irreversible(IrvLiftingStep { a: GAMMA }));
        self.steps.push(LiftingStep::Irreversible(IrvLiftingStep { a: DELTA }));
    }

    /// Initializes for the standard reversible 5/3 wavelet.
    pub fn init_rev53(&mut self) {
        // 5/3 lifting: 2 steps
        // Step 0 (predict): a=-1, b=1, e=1  → d[n] -= (s[n-1]+s[n]) >> 1
        // Step 1 (update):  a=1,  b=2, e=2  → s[n] += (d[n-1]+d[n]+2) >> 2
        self.natk = 2;
        self.katk = 0.0;
        self.steps.clear();
        self.steps.push(LiftingStep::Reversible(RevLiftingStep {
            a: -1,
            b: 1,
            e: 1,
        }));
        self.steps.push(LiftingStep::Reversible(RevLiftingStep {
            a: 1,
            b: 2,
            e: 2,
        }));
    }
}

// =========================================================================
// Function pointer types — wavelet transforms
// =========================================================================

/// Reversible / irreversible vertical lifting step.
pub type RevVertStepFn = fn(
    s: &LiftingStep,
    sig: &LineBuf,
    other: &LineBuf,
    aug: &mut LineBuf,
    repeat: u32,
    synthesis: bool,
);

/// Reversible horizontal analysis (forward DWT, split into low/high).
pub type RevHorzAnaFn = fn(
    atk: &ParamAtk,
    ldst: &mut LineBuf,
    hdst: &mut LineBuf,
    src: &LineBuf,
    width: u32,
    even: bool,
);

/// Reversible horizontal synthesis (inverse DWT, merge low/high).
pub type RevHorzSynFn = fn(
    atk: &ParamAtk,
    dst: &mut LineBuf,
    lsrc: &mut LineBuf,
    hsrc: &mut LineBuf,
    width: u32,
    even: bool,
);

/// Irreversible vertical lifting step (same shape as reversible).
pub type IrvVertStepFn = fn(
    s: &LiftingStep,
    sig: &LineBuf,
    other: &LineBuf,
    aug: &mut LineBuf,
    repeat: u32,
    synthesis: bool,
);

/// Multiply line by normalization constant K.
pub type IrvVertTimesKFn = fn(k: f32, aug: &mut LineBuf, repeat: u32);

/// Irreversible horizontal analysis.
pub type IrvHorzAnaFn = fn(
    atk: &ParamAtk,
    ldst: &mut LineBuf,
    hdst: &mut LineBuf,
    src: &LineBuf,
    width: u32,
    even: bool,
);

/// Irreversible horizontal synthesis.
pub type IrvHorzSynFn = fn(
    atk: &ParamAtk,
    dst: &mut LineBuf,
    lsrc: &mut LineBuf,
    hsrc: &mut LineBuf,
    width: u32,
    even: bool,
);

/// Runtime-dispatched wavelet transform function table.
pub struct WaveletTransformFns {
    pub rev_vert_step: RevVertStepFn,
    pub rev_horz_ana: RevHorzAnaFn,
    pub rev_horz_syn: RevHorzSynFn,
    pub irv_vert_step: IrvVertStepFn,
    pub irv_vert_times_k: IrvVertTimesKFn,
    pub irv_horz_ana: IrvHorzAnaFn,
    pub irv_horz_syn: IrvHorzSynFn,
}

// =========================================================================
// Function pointer types — colour transforms
// =========================================================================

/// Reversible sample conversion (integer shift).
pub type RevConvertFn = fn(
    src_line: &LineBuf,
    src_line_offset: u32,
    dst_line: &mut LineBuf,
    dst_line_offset: u32,
    shift: i64,
    width: u32,
);

/// Irreversible: float → integer quantization.
pub type IrvConvertToIntegerFn = fn(
    src_line: &LineBuf,
    dst_line: &mut LineBuf,
    dst_line_offset: u32,
    bit_depth: u32,
    is_signed: bool,
    width: u32,
);

/// Irreversible: integer → float dequantization.
pub type IrvConvertToFloatFn = fn(
    src_line: &LineBuf,
    src_line_offset: u32,
    dst_line: &mut LineBuf,
    bit_depth: u32,
    is_signed: bool,
    width: u32,
);

/// RCT forward/backward (integer buffers).
pub type RctFn = fn(
    c0: &LineBuf,
    c1: &LineBuf,
    c2: &LineBuf,
    d0: &mut LineBuf,
    d1: &mut LineBuf,
    d2: &mut LineBuf,
    repeat: u32,
);

/// ICT forward/backward (float buffers).
pub type IctFn = fn(
    c0: &[f32],
    c1: &[f32],
    c2: &[f32],
    d0: &mut [f32],
    d1: &mut [f32],
    d2: &mut [f32],
    repeat: u32,
);

/// Runtime-dispatched colour transform function table.
pub struct ColourTransformFns {
    pub rev_convert: RevConvertFn,
    pub rev_convert_nlt_type3: RevConvertFn,
    pub irv_convert_to_integer: IrvConvertToIntegerFn,
    pub irv_convert_to_float: IrvConvertToFloatFn,
    pub irv_convert_to_integer_nlt_type3: IrvConvertToIntegerFn,
    pub irv_convert_to_float_nlt_type3: IrvConvertToFloatFn,
    pub rct_forward: RctFn,
    pub rct_backward: RctFn,
    pub ict_forward: IctFn,
    pub ict_backward: IctFn,
}

// =========================================================================
// Runtime dispatch — OnceLock singletons
// =========================================================================

static WAVELET_FNS: OnceLock<WaveletTransformFns> = OnceLock::new();
static COLOUR_FNS: OnceLock<ColourTransformFns> = OnceLock::new();

/// Initializes wavelet transform function pointers (called once, lazily).
pub fn init_wavelet_transform_functions() -> &'static WaveletTransformFns {
    WAVELET_FNS.get_or_init(|| {
        // Start with generic implementations.
        // Future phases will add SIMD dispatch here.
        WaveletTransformFns {
            rev_vert_step: wavelet::gen_rev_vert_step,
            rev_horz_ana: wavelet::gen_rev_horz_ana,
            rev_horz_syn: wavelet::gen_rev_horz_syn,
            irv_vert_step: wavelet::gen_irv_vert_step,
            irv_vert_times_k: wavelet::gen_irv_vert_times_k,
            irv_horz_ana: wavelet::gen_irv_horz_ana,
            irv_horz_syn: wavelet::gen_irv_horz_syn,
        }
    })
}

/// Initializes colour transform function pointers (called once, lazily).
pub fn init_colour_transform_functions() -> &'static ColourTransformFns {
    COLOUR_FNS.get_or_init(|| {
        // Start with generic implementations.
        ColourTransformFns {
            rev_convert: colour::gen_rev_convert,
            rev_convert_nlt_type3: colour::gen_rev_convert_nlt_type3,
            irv_convert_to_integer: colour::gen_irv_convert_to_integer,
            irv_convert_to_float: colour::gen_irv_convert_to_float,
            irv_convert_to_integer_nlt_type3: colour::gen_irv_convert_to_integer_nlt_type3,
            irv_convert_to_float_nlt_type3: colour::gen_irv_convert_to_float_nlt_type3,
            rct_forward: colour::gen_rct_forward,
            rct_backward: colour::gen_rct_backward,
            ict_forward: colour::gen_ict_forward,
            ict_backward: colour::gen_ict_backward,
        }
    })
}

/// Returns a reference to the lazily-initialized wavelet function table.
#[inline]
pub fn wavelet_fns() -> &'static WaveletTransformFns {
    init_wavelet_transform_functions()
}

/// Returns a reference to the lazily-initialized colour function table.
#[inline]
pub fn colour_fns() -> &'static ColourTransformFns {
    init_colour_transform_functions()
}
