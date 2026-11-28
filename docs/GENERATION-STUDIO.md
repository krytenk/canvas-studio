# Generation studio — 1.19.0

Canvas Studio has separate **Design**, **Generate** and **Video studio** workspaces.
Generate works in desktop, browser and MCP. It searches model metadata, prepares
private finite Kaggle notebooks, submits configured runs, downloads completed
assets and imports them as ordinary editable-project images or Film footage.
Designing, templates and video editing remain usable without model services.

## Browser connection and search

Open the complete private session URL supplied when the local browser editor
starts, including its `#…` fragment. Browser sessions are separate: copying only
`http://127.0.0.1:PORT/` into a different browser does not authorize that browser.
Version 1.16.1 shows a connection screen for this case. Paste the complete link
there, or follow the complete link in the same tab; entered fields are preserved.
Provider API keys cannot replace this local session link. Token/origin checks are
still required for all private API calls.

In Generate, the upper filter searches the reviewed library only. Open **Discover
models & weights**, choose Hugging Face/Civitai, enter a query and click **Search
live catalog**. Progress, results, zero matches and provider errors appear beside
that form. Task history also labels the provider and query. Connections lists
which credentials are saved; this is not a claim that a provider has validated
those credentials. Search/registration uses the saved provider token where needed.

## Everyday use

Desktop and browser now use the same short workflow:

1. Open **Generate**, choose **Images** or **Video**, and select a model.
2. Describe what you want. Add photos if the selected model uses them, or select
   a saved character; saved photos load immediately.
3. Click **Generate**. The studio submits a private Kaggle job, monitors it and
   downloads the verified result automatically. The ready image/video appears in
   **Your creations**, with **Use in design / Use in video** and **Download**.

You do not need to prepare a notebook, submit it separately, check its status or
fetch its finished output. Recommended model settings are selected automatically.
RealVisXL/SDXL's simple image preset uses the reviewed four-step Lightning adapter;
**More options** retains editable resolution, precision, seed and other settings.
Notebook preparation, local inference and manual submission are developer tools
inside that panel. Connection settings and technical history start collapsed when
an existing connection is configured. Neither interface requires naming/saving a
reference set before generating.

The one-time setup is still needed on a new computer: install the small provider
tools with `python3 scripts/setup_generation.py`, then save your Kaggle profile
handle and API token in **Connection settings**. Existing saved keys are reused.
Google sign-in is fine: create the separate token at
[Kaggle API settings](https://www.kaggle.com/settings/api); do not enter a Google
password in the studio.

Generation runs in the local background worker, rather than depending on the
browser's status button or an open Generate panel. Editing remains available.
Progress shows preparing, waiting for a GPU, generating and saving; it does not
invent a percentage for unmeasured model work. Duplicate Generate clicks for an
active document are rejected. Remote waits use their own worker slots so they do
not consume the local film-render slots.

If a connection fails repeatedly, **Continue** checks the recorded run and resumes
its download. It does not submit another GPU job. An unconfirmed submission is
recorded for inspection rather than automatically retried. Cancellation under
Technical history stops local waiting; stop a remote GPU notebook in Kaggle itself.
GPU availability, queueing and your account allowance still determine the wait.

For gated remote weights, accept the publisher's licence using your own provider
account and add `HF_TOKEN` / `CIVITAI_API_KEY` in **Kaggle Secrets**, making those
secrets available to the notebook. Local credentials are deliberately not embedded
in downloaded notebooks. Runs never create public inference tunnels or persistent
GPU servers. **Cancel local task does not stop a submitted Kaggle notebook**;
stop that remote run using Kaggle's notebook controls.

## Model review, checked 27 November 2026

This is a practical shortlist by purpose, licence and hardware fit, not an
independent quality ranking or a promise that every newly released model works.
All curated entries have free-to-access weights (some require licence acceptance)
and a pinned publisher revision. Free weights do not mean unlimited free compute
or unrestricted commercial rights. Read each card and licence for your use.

| Model | Useful role | Licence / runner status |
| --- | --- | --- |
| [FLUX.2 Klein 4B](https://huggingface.co/black-forest-labs/FLUX.2-klein-4B) | Modern images and explicit reference-page editing; publisher suggests ~13 GB VRAM | Apache 2.0; recipe provided |
| [Z-Image Turbo](https://huggingface.co/Tongyi-MAI/Z-Image-Turbo) | Fast photorealistic image work; publisher targets 16 GB devices | Apache 2.0; recipe provided |
| [RealVisXL 5](https://huggingface.co/SG161222/RealVisXL_V5.0) | Practical fashion, portrait and photographic SDXL work | OpenRAIL++; recipe provided |
| [SDXL 1.0](https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0) | Established image baseline and compatible adapter workflows | OpenRAIL++; recipe provided |
| [Wan 2.1 1.3B](https://huggingface.co/Wan-AI/Wan2.1-T2V-1.3B-Diffusers) | First GPU video test; publisher reports 8.19 GB baseline | Apache 2.0; recipe provided |
| [Wan 2.2 TI2V 5B](https://huggingface.co/Wan-AI/Wan2.2-TI2V-5B-Diffusers) | Higher-weight video | Apache 2.0; text-to-video recipe provided; 24 GB planning target |
| [Qwen Image 2512](https://huggingface.co/Qwen/Qwen-Image-2512) | Heavy image generation | Apache 2.0; recipe provided; substantial GPU/RAM needs |
| [Qwen Image 2.1](https://huggingface.co/Qwen/Qwen-Image-2.1) | Personal research, image generation/reference editing | Research licence; advertising blocked |
| [Qwen 2.1 Uncensored Omni](https://huggingface.co/chatrlabs/Qwen-Image-2.1-Uncensored-Omni-Quantized) | Publisher-declared uncensored comparison | Research licence; discovery only; dedicated quantized recipe needed |
| [LTX 2.5](https://huggingface.co/Lightricks/LTX-2.5-Diffusers) | Heavy video exploration | Gated community licence; review required; video recipe provided |
| [Krea 2 Raw](https://huggingface.co/krea/Krea-2-Raw) | Photographic image exploration | Gated community licence; review required; recipe provided |
| [MiniMax H3 Image](https://huggingface.co/MiniMaxAI/MiniMax-H3-Image) | Recent high-weight image research | Custom licence/code; discovery only |
| [Nemotron Labs Image](https://huggingface.co/nvidia/Nemotron-Labs-Image-30B-A3B) | Recent large-model research | Custom licence/recipe; discovery only |
| [HunyuanVideo 1.5](https://huggingface.co/tencent/HunyuanVideo-1.5) | Video research | Community licence/original layout; discovery only |

VRAM numbers in the interface are planning estimates unless explicitly attributed
to a publisher above. Resolution, frames, activations, offload and runtime versions
change actual use. **Two T4s do not pool their VRAM automatically.** Larger recipes
are selectable for preparation/research, with hardware warnings, rather than sold
as guaranteed T4 fits. Precision is recipe-specific: SDXL defaults to FP16; Wan 2.1 defaults to
BF16 with an FP32 decoder. BF16 checks the CUDA runtime and executes a small
CUDA matrix operation before downloading weights. CPU requires FP32 and is very slow for real models.

**NF4** quantizes supported Diffusers transformer components while loading; it
does not magically shrink the original download. SDXL/SD1.5 full checkpoints use
FP16/FP32 rather than this transformer NF4 recipe. CPU offload trades GPU memory
for RAM and transfer time. The pinned [SDXL Lightning 4-step LoRA](https://huggingface.co/ByteDance/SDXL-Lightning)
uses compatible SDXL recipes, four steps and zero guidance. Other adapters must
receive a reviewed compatible recipe before use.

## Content selection and live discovery

The selector distinguishes open-weight recipes, publisher defaults and
**publisher-declared uncensored** models. Civitai's `nsfw` listing tag does not
prove whether a model refuses a prompt, so it is not labelled as a censorship
test. The **include mature metadata** checkbox controls metadata discovery only;
the app does not automatically fetch gallery images.

**Flexible** preserves model/provider defaults without an extra application
classifier. **Standard** adds a pinned adult-content output classifier, checking
every generated video frame. This is a fallible content check, not a guarantee.
Neither choice removes a model safety checker or changes a provider's terms.
The intended workflow includes lawful adult fashion and influencer imagery.

Live Hugging Face/Civitai search prevents the catalog becoming a fixed snapshot.
HF registration requires a supported Diffusers pipeline and safetensors;
Civitai registration currently accepts public full SDXL/SD1.5 safetensors
checkpoints with a publisher SHA-256. Paid/early-access-only weights and pickle
checkpoints are rejected. Civitai LoRAs are discoverable but need a compatible
adapter recipe; arbitrary custom Python is not executed. Newly registered
licences remain marked for review rather than automatically approved for ads.

## Kaggle runtime and remote status

Version 1.16.2 matches notebook titles to unique slugs and records the canonical
URL and version returned by Kaggle. A local submission task completing is shown
as **GPU / submitted**, not a claim that the image or video has finished. Use
**Check GPU run** to see queued/running/complete/error states. Failures include a
readable, redacted execution-log tail. A request timeout is reported as unconfirmed;
check that run before retrying, since the remote notebook may already exist.

The finite notebook removes an incompatible optional `torchao` package found in
the tested Kaggle GPU image, then installs pinned PEFT 0.22.0 and bitsandbytes 0.50.2.
This affects only the disposable remote environment, not local desktop packages.
The runner's NF4 option uses bitsandbytes. Wan VAE decoding loads in FP32 while
its denoiser uses the selected precision; non-finite video pixels are rejected before export.
See the [publisher's Wan pipeline recipe](https://huggingface.co/docs/diffusers/en/api/pipelines/wan).

Status, logs and output requests target the recorded notebook version. Downloads
must match the recorded model/revision and their receipt checksum before import.
Selecting an incompatible model clears the previous model's adapter.

## Image to video and uploaded references

Choose **Generate → Video → LTX Video** in desktop or browser, then choose a first
frame. PNG, JPEG and WebP uploads are limited to 25 MB / 16 megapixels. The app
applies EXIF rotation, strips source metadata, normalizes to an immutable PNG and
limits the longest edge to 2048 pixels. Uploading does not add or replace a design
layer. The reference is scoped to this document and previewed before submission.
Selecting a replacement clears the old selection immediately; failed uploads
cannot silently reuse a previous influencer. Submission waits for upload completion.

The initial practical recipe is [LTX Video 2B 0.9.5](https://huggingface.co/Lightricks/LTX-Video-0.9.5),
pinned to `e58e28c39631af4d1468ee57a853764e11c1d37e`. This is a small first-frame
baseline with Open RAIL-M terms, not a claim of leading current video quality.
Its starting settings are BF16, CPU offload, 384 × 512, 49 frames, 24 fps,
30 steps, guidance 3 and a 32 GiB download allowance. The model downloads about
23.65 GiB. It requires `8n+1` frames; text-only video recipes retain their own defaults.
The runner passes the chosen frame rate and model-specific decode settings.
See the [Diffusers LTX recipe](https://huggingface.co/docs/diffusers/en/api/pipelines/ltx_video).

**Keep whole image** preserves the composition with padded edges. **Fill frame**
crops the edges to the output aspect ratio. Match the video dimensions to a
portrait for influencer footage. Describe modest subject/camera motion; a first
frame conditions the clip but does not guarantee perfect identity in every frame.
Alternatively select **Current design page** to animate a composed page snapshot.
Text-only recipes never receive a retained reference selection.

Kaggle rejects large notebook source, so reference pixels travel as a separate
**private input dataset**, not an inline base64 image. The app uses the official
CLI's private creation path, retains rights under an `other` metadata label,
checks actual remote privacy, reuses the same hashed input on retries and pins
its dataset version. Private inputs remain on Kaggle until you remove them there;
clearing a local reference or task does not delete remote data. Preparing a ZIP
is offline: it includes the lossless reference file and a small notebook; app
submission attaches the private input automatically. Manual use requires attaching
the matching reference as a private input. Neither notebooks nor ZIPs embed keys.

The receipt records the source image checksum, prepared first-frame checksum,
framing and dimensions. Download verifies the reference fingerprint as well as
model/revision and output hash. Completed MP4s import into Video studio normally. New generated shots append after
existing visual clips and extend the film when needed, retaining the full clip.
Explicit MCP `start` positions remain available with normal overlap validation.

The live editor MCP uploads a scoped workspace file with `studio_generation`
`operation: "reference", reference_path: "portrait.jpg"`; use the returned
`reference_id` with prepare/submit/generate and `reference_fit: "contain"` or
`"cover"`. Paths are allowed only for this explicit upload operation. The standalone
Kaggle adapter can use an already uploaded reference ID with the same document ID.

## Multiple subject references and consistent shots

Choose **Generate → Video → Wan VACE** in desktop or browser. Upload two to four
views of the **same adult character**, ideally with consistent hair, wardrobe and
lighting. Each image stays separate: the app does not build a collage or substitute
timed keyframes. Add/remove/reorder views before submission. A single-image recipe
rejects a larger selection instead of silently dropping images.

Save a named set (for example, “Character · portrait and side views”) to the private
local library. Load it into this or another document for subsequent shots. Saving
copies the normalized images into the library, so it survives removal of the
original document's review history. Set names and ordering are immutable snapshots;
changing a selection and saving creates another snapshot. There are at most 100
saved sets, each containing up to four images. Library files are private local
files outside Git and editable project bundles, not a public model-training dataset.

The first reviewed implementation is
[Wan 2.1 VACE 1.3B](https://huggingface.co/Wan-AI/Wan2.1-VACE-1.3B-diffusers),
Apache 2.0, revision `ec4d2cb062b548996b179d493fdd05340de702a1`. It uses
[Diffusers' subject-reference inputs](https://huggingface.co/docs/diffusers/en/api/pipelines/wan)
with all images in one video batch. The starting preset is 384 × 512, 17 frames,
8 fps, 20 steps, guidance 5, BF16 denoiser, FP32 VAE, CPU offload and a 24 GiB
allowance for about 17.73 GiB of weights. **Reference influence** controls VACE's
aggregate conditioning scale (0.5–2); it is not a per-face identity score.

Keep the same ordered set across shots, change your shot direction and use a
fresh seed as needed. This conditions the model on more views but **does not lock
identity**. The small free-GPU model can alter faces, hair, clothing or proportions;
review every shot before use. More references also increase sequence length and
runtime. First-frame image-to-video remains a separate workflow.

Prepared ZIPs include each lossless normalized reference and a small notebook.
Kaggle submission attaches one private, version-pinned input pack, reused for the
same ordered set. Downloads verify **every ordered source hash**, the complete
set fingerprint, model revision and output hash. A missing, substituted or
reordered reference causes rejection rather than an apparently successful import.

MCP: upload each workspace image using `studio_generation` operation `reference`,
then send `reference_ids: [id1, id2]` to prepare/submit/generate. Operations
`save_reference_set` (`reference_set_name`, `reference_ids`), `reference_sets` and
`use_reference_set` (`reference_set_id`) support the same local library. Loading
returns document-scoped IDs; pass them to subsequent jobs. The original singular
`reference_id` remains compatible. No provider credentials belong in tool arguments.

## Storage, reproducibility and limits

- Default tool/runtime/cache location is `~/.local/share/canvas-studio/`, outside
  the repository. Set an absolute cache folder in Connections to use an external
  drive. Model weights, credentials, generated jobs and venvs are not Git assets.
- Submission checks pinned model file sizes against the download budget before
  reserving a GPU notebook. Wan 2.1 1.3B downloads about 26.94 GiB of original
  weights; its default download allowance is 32 GiB. Precision conversion does
  not reduce the original weight download. A 3 GiB cache reserve is checked
  again inside the runtime. Remote weights use ephemeral `/kaggle/temp` storage,
  outside persisted notebook outputs, and are cleaned up after execution.
  Safetensors variants are selected without duplicate full checkpoints. Partial
  Civitai downloads are process-specific and checksum verified.
- Inference receipts include pinned model, settings, seed, versions, device,
  elapsed time and output hash. Notebook manifests include runtime source hash.
- Image generation is bounded to 256–1536 pixels per side; video to 9–121 frames,
  8/16/24 fps; inference steps to 1–60. Image import is limited to 64 MB/16 MP.
- Generated MP4 imports are rendered footage, not editable neural-model internals.
  LTX's native audio capabilities are not exposed by this first video-frame runner.
- Design/video render history and generation history have separate clear controls.
  Save outputs first; clearing review tasks preserves imported project assets, model
  caches and submitted remote notebooks.
- Provider calls are asynchronous and local request workers can be cancelled.
  No service credentials are returned by the application or MCP.

## Verification and remaining work

`scripts/generation_tests.py` covers private notebooks, references, rights gates,
bounded GPU submission, document isolation, checksum failures, discovery and secret
redaction. `scripts/generation_smoke.py` exercises the real app/MCP/browser path.
Its optional `--local-proof` performs actual CPU safetensors inference through the
worker and verifies import, undo/redo and portable cold-open, using tiny random
test weights. This is integration evidence, not an image-quality demonstration.

Two-shot VACE multi-reference acceptance also passed real private T4 execution,
complete-set receipt checks, visual review, sequential import, playback and cold
portable recovery. See the acceptance report for timings and identity limitations.

RealVisXL + SDXL Lightning images and Wan 2.1 1.3B video have passed real private
Kaggle Tesla T4 runs, download, page/timeline import and video playback. The tested
512 × 320 / 33-frame / 20-step Wan clip took 468.64 seconds of inference, plus
setup, weights and queueing. See the scoped
[GPU acceptance report](KAGGLE-GPU-ACCEPTANCE.md). Production recipes still need
individual account/hardware validation. Per-model identity benchmarks, character-training adapters, arbitrary community LoRA stacks, multi-GPU sharding and remote stop automation
remain future work. No claim of having benchmarked every production recipe is made.

Primary integration references: [Kaggle CLI notebook workflow](https://github.com/Kaggle/kaggle-cli/blob/main/skills/references/kernels.md),
[Diffusers quantization](https://huggingface.co/docs/diffusers/quantization/overview),
[Civitai models API](https://github.com/civitai/civitai-developer-docs/blob/main/site/reference/models.md).
