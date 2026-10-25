/* Handy Local extension for Qwen3-ASR's system-context text. */
#ifndef TRANSCRIBE_QWEN3_ASR_H
#define TRANSCRIBE_QWEN3_ASR_H
#include "transcribe.h"

/* 'Q3RN', a private fork extension; does not change the generic ABI. */
#define TRANSCRIBE_EXT_KIND_QWEN3_ASR_RUN 0x4E523351u
struct transcribe_qwen3_asr_run_ext {
    struct transcribe_ext ext;
    /* UTF-8 context, borrowed only for run/run_batch. NULL means empty.
       Limited to 4096 bytes / 512 tokens; chat control markers are rejected. */
    const char * context;
};
#endif
