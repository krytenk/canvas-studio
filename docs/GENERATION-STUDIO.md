# Generation studio — 1.16.2

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

## Start here

1. Install the small provider tools: `python3 scripts/setup_generation.py`.
2. Open **Generate → Connections & storage**. Enter your Kaggle username and API
   token there, never in a prompt or chat. Google sign-in works normally: create
   the separate token at [Kaggle API settings](https://www.kaggle.com/settings/api).
   Use your profile handle as the username, not your Google email or password. Tokens are held in a private local file
   outside projects and Git. Optional Hugging Face/Civitai credentials support
   authenticated local downloads and metadata access.
3. Start with RealVisXL/SDXL for portraits or Wan 2.1 1.3B for modest video tests.
   Choose a prompt, resolution, seed and memory settings. **Prepare private GPU
   notebook** works before connecting Kaggle and gives you an inspectable ZIP.
4. **Submit to Kaggle** uploads the prompt and any explicitly selected reference
   page into a private notebook, with a selected 15/30/60 minute limit and a T4
   accelerator request. GPU quota, account eligibility and availability are
   controlled by Kaggle. There is no paid endpoint or automatic wallet top-up.
5. Check the run, download the finished asset, then place it on the design page or
   add it to the video edit. Ordinary undo, portable saving and later editing work.

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

RealVisXL + SDXL Lightning image execution has passed a real private Kaggle
Tesla T4 run, download and page import. See the scoped
[GPU acceptance report](KAGGLE-GPU-ACCEPTANCE.md). Production recipes still need
individual account/hardware validation. Per-model quality benchmarks, uploaded reference-photo UI,
arbitrary community LoRA stacks, multi-GPU sharding and remote stop automation
remain future work. No claim of having benchmarked every production recipe is made.

Primary integration references: [Kaggle CLI notebook workflow](https://github.com/Kaggle/kaggle-cli/blob/main/skills/references/kernels.md),
[Diffusers quantization](https://huggingface.co/docs/diffusers/quantization/overview),
[Civitai models API](https://github.com/civitai/civitai-developer-docs/blob/main/site/reference/models.md).
