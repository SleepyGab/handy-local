// Handy Local additive extension. Matches include/transcribe/qwen3_asr.h.
pub const TRANSCRIBE_EXT_KIND_QWEN3_ASR_RUN: u32 = 0x4E523351;
#[repr(C)]
pub struct transcribe_qwen3_asr_run_ext {
    pub ext: crate::transcribe_ext,
    pub context: *const std::os::raw::c_char,
}
