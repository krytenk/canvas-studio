# Kaggle GPU acceptance — 27 November 2026

This report records real private Kaggle execution initiated from the browser
Generation studio, using the same native worker/backend as the desktop app.
It is separate from the offline CI fixtures and tiny random CPU integration test.
No provider credentials are included in the notebooks, repository or this report.

## Image — passed

- Model: `SG161222/RealVisXL_V5.0`, pinned catalog revision, compatible pinned
  SDXL Lightning four-step adapter; FP16, CPU offload, 512 × 512, four steps,
  guidance 0, seed 20261127. Prompt: a green linen notebook on a sunlit oak desk.
- Real Tesla T4; Torch 2.11.0+cu130; Diffusers 0.43.0, Transformers 5.24.1,
  PEFT 0.22.0, Accelerate 1.16.0, bitsandbytes 0.50.2.
- Inference receipt: 30.85 seconds, excluding provisioning, packages and weights.
- Studio status reached complete; Download finished asset produced the PNG;
  Place on current page added an editable image element. The file was decoded,
  its dimensions checked and its appearance inspected. The model produced small
  incidental cover lettering despite the no-text prompt: layout typography should
  still be authored as editable text in the studio.
- PNG SHA-256: `98472e681dca1350d4182bf647f02c23a0dc9e3533ccbe4be6f953fde9e8efa5`.

## Issues found and repaired

1. Kaggle canonicalized an earlier descriptive notebook title into a different
   URL. Notebook titles now match unique requested slugs; the confirmed canonical
   handle and version are recorded and used for status, logs and downloads.
2. The SDK can report push errors with a zero process exit. Submission now needs
   a confirmed success and owner-matching canonical URL. Timeouts remain
   unconfirmed and checkable without silently creating duplicate GPU jobs.
3. The Kaggle base image had optional torchao 0.10.0, incompatible with the
   installed PEFT. The disposable notebook removes it and pins PEFT/bitsandbytes.
4. Wan's original pipeline weights need 26.94 GiB, exceeding the previous 20 GiB
   allowance. Model sizes are now checked before GPU submission; the reviewed Wan
   preset uses 32 GiB and BF16. It keeps the VAE in FP32 and probes BF16 CUDA work
   before downloading. Desktop, browser and agent requests share these defaults.
5. Remote model caches now live under ephemeral `/kaggle/temp`, outside persisted
   outputs, and are removed after execution. Imported results must match the
   model/revision and checksum. Incompatible image adapters clear on video selection.

## Scope

The image is an actual production-model acceptance result, not a general
benchmark. Queueing and downloads add substantial time. No paid compute or
public notebook publication was used. Other catalog recipes, multi-GPU sharding,
remote cancellation and arbitrary community adapters remain separately unverified.
