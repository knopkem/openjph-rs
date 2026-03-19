//! JPEG 2000 codestream parser and generator.
//!
//! Port of `ojph_codestream.h/cpp`. The [`Codestream`] struct is the main
//! entry point for encoding and decoding HTJ2K images.

pub mod bitbuffer_read;
pub mod bitbuffer_write;
pub(crate) mod local;
pub(crate) mod tile;
pub(crate) mod tile_comp;
pub(crate) mod resolution;
pub(crate) mod subband;
pub(crate) mod precinct;
pub(crate) mod codeblock;
pub(crate) mod codeblock_fun;
pub(crate) mod simd;

use crate::error::Result;
use crate::file::{OutfileBase, InfileBase};
use crate::params::{ParamSiz, ParamCod, ParamQcd, ParamNlt, CommentExchange};

/// The main codestream interface for encoding and decoding HTJ2K images.
///
/// This is the public wrapper over the internal [`local::CodestreamLocal`].
/// It mirrors the C++ `ojph::codestream` class (which used a pImpl pattern).
#[derive(Debug, Default)]
pub struct Codestream {
    inner: local::CodestreamLocal,
}

impl Codestream {
    /// Create a new codestream with default parameters.
    pub fn new() -> Self {
        Self {
            inner: local::CodestreamLocal::new(),
        }
    }

    /// Reset the codestream for reuse.
    pub fn restart(&mut self) {
        self.inner.restart();
    }

    // ----- Parameter access -----

    /// Access the SIZ (image/tile size) parameters.
    pub fn access_siz(&self) -> &ParamSiz {
        self.inner.access_siz()
    }

    /// Access the SIZ parameters for modification.
    pub fn access_siz_mut(&mut self) -> &mut ParamSiz {
        self.inner.access_siz_mut()
    }

    /// Access the COD (coding style) parameters.
    pub fn access_cod(&self) -> &ParamCod {
        self.inner.access_cod()
    }

    /// Access the COD parameters for modification.
    pub fn access_cod_mut(&mut self) -> &mut ParamCod {
        self.inner.access_cod_mut()
    }

    /// Access the QCD (quantization) parameters.
    pub fn access_qcd(&self) -> &ParamQcd {
        self.inner.access_qcd()
    }

    /// Access the QCD parameters for modification.
    pub fn access_qcd_mut(&mut self) -> &mut ParamQcd {
        self.inner.access_qcd_mut()
    }

    /// Access the NLT (nonlinearity) parameters.
    pub fn access_nlt(&self) -> &ParamNlt {
        self.inner.access_nlt()
    }

    /// Access the NLT parameters for modification.
    pub fn access_nlt_mut(&mut self) -> &mut ParamNlt {
        self.inner.access_nlt_mut()
    }

    // ----- Configuration -----

    /// Enable resilient (error-tolerant) mode for decoding.
    pub fn enable_resilience(&mut self) {
        self.inner.enable_resilience();
    }

    /// Set planar mode (0 = interleaved, 1 = planar).
    pub fn set_planar(&mut self, planar: i32) {
        self.inner.set_planar(planar);
    }

    /// Set the codestream profile (e.g., "IMF", "BROADCAST").
    pub fn set_profile(&mut self, name: &str) -> Result<()> {
        self.inner.set_profile(name)
    }

    /// Set tilepart division flags.
    pub fn set_tilepart_divisions(&mut self, value: u32) {
        self.inner.set_tilepart_divisions(value);
    }

    /// Request that TLM markers be written.
    pub fn request_tlm_marker(&mut self, needed: bool) {
        self.inner.request_tlm_marker(needed);
    }

    /// Restrict the number of resolution levels used for reading/reconstruction.
    pub fn restrict_input_resolution(
        &mut self,
        skipped_res_for_data: u32,
        skipped_res_for_recon: u32,
    ) {
        self.inner.restrict_input_resolution(skipped_res_for_data, skipped_res_for_recon);
    }

    // ----- Write path -----

    /// Write codestream headers to the output file.
    pub fn write_headers(
        &mut self,
        file: &mut dyn OutfileBase,
        comments: &[CommentExchange],
    ) -> Result<()> {
        self.inner.write_headers(file, comments)
    }

    // ----- Read path -----

    /// Read codestream headers from the input file.
    pub fn read_headers(&mut self, file: &mut dyn InfileBase) -> Result<()> {
        self.inner.read_headers(file)
    }

    // ----- Query -----

    /// Returns true if planar mode is active.
    pub fn is_planar(&self) -> bool {
        self.inner.is_planar()
    }

    /// Returns the number of tiles in x and y.
    pub fn get_num_tiles(&self) -> crate::types::Size {
        self.inner.num_tiles
    }
}
