# Personal hosting and a future public service

The initial publishing edition uses CPU design/editing/export tools and personal
Research. Image/video generation is excluded. RunPod can be considered later
when an explicit funding allowance supports it; it is not required to use the
editor and is not connected by this milestone.

## What a VPS would run

| Component | Personal VPS role | Notes |
| --- | --- | --- |
| Browser interface and design engine | Serve the interface, keep the active project and run the shared engine | Current editor is a private, single-user loopback service |
| Project files and imported media | Store portable projects, research snapshots and originals | Back up independently from disposable render/proxy caches |
| PDF/PNG and video editing exports | Run bounded CPU workers | FFmpeg and optional Chromium/motion dependencies are needed; benchmark real workloads |
| Transcription | Run supported CPU speech recognition, if Reference Desk is installed there | Slower than a GPU; uploading existing transcripts avoids repeating it |
| Future AI generation | Call a separately funded external GPU endpoint | No GPU is needed on the VPS when inference runs elsewhere |

A machine with eight CPU cores and 24 GB RAM is a plausible starting point for
one personal studio and bounded export work. This is a capacity assessment, not
a benchmark or guarantee. Check operating system, free disk, free RAM, current
CPU load and existing websites before reserving resources. A 200 GB disk can
fill quickly with original videos, proxies, renders and backups. Start with one
heavy export at a time and a measured storage allowance; clear disposable jobs
through the app after saving deliverables.

Paying for a VPS already can make it economical to use spare capacity before
buying another general-purpose server. A fast network link helps transfer
files; it does not provide GPU acceleration.

CanSpace advertises root access, Canadian infrastructure and managed server
maintenance. Its onboarding guide distinguishes managed operating-system work
from responsibility for the application's code, databases and content. Confirm
the actual account's backup coverage and retention rather than assuming every
plan has the same restore arrangements. [CanSpace VPS](https://www.canspace.ca/vps.html),
[management responsibilities](https://canspace.ca/clients/knowledgebase/95/Getting-started-with-your-VPS-or-dedicated-server.html).

## Personal access first

The safest first hosting milestone is a private personal installation. Keep the
engine bound to loopback and access it through a reviewed private SSH/VPN route.
An SSH forward needs to preserve the loopback host/port expected by the editor.
Keep its private session URL private. Do not open the engine's port directly to
the internet or bypass its Host/Origin guards to make a public domain work.

Deploy under a dedicated unprivileged account and service with bounded resources,
not as root or inside an existing site's writable web directory. Use a compatible
build/container: the current Linux release targets Ubuntu 24.04 and may not run
unchanged on the VPS's operating system. Check FFmpeg, fonts, Chromium/motion
dependencies and disk permissions on the actual host. Keep a known-good local
build and test restores before moving essential work.

Moving the editor moves the data location. A server cannot see a client's
`~/Reference Desk` folder. Upload selected TXT/SRT/audio/video files through
Research, or later configure an explicit private sync of selected outputs. Do
not synchronise an entire home folder. Projects downloaded from the server remain
portable and can be opened by a compatible local desktop version.

No server login, DNS changes, software installation or public deployment is
performed by this implementation. A host-readiness inspection is the next step
once a private hosting move is requested.

## Publishing profile

`run.sh`, the desktop launcher and `run-browser.sh` default to
`CANVAS_STUDIO_PROFILE=publishing`. Home offers product design, campaigns, video
editing and Research. Generation navigation is hidden and generation API/worker
requests are rejected before inference starts. No model keys are required.

The existing optional personal-generation tools can be selected explicitly with
`CANVAS_STUDIO_PROFILE=personal`. Direct binary launches also leave generation
disabled unless explicitly selected. Integration checks opt in to exercise the
optional tools. This is an owner's deployment feature switch, not subscription
authentication. It does
not make the current single-user server a public SaaS.

## A public website is a separate milestone

Public access needs HTTPS, application authentication, private per-user storage,
document isolation, resource quotas, rate limits, safe worker isolation and
backup/deletion/restore tests. The personal Reference Desk file bridge and local
MCP must not become public administrative interfaces. Public customers should
upload their own files or use authorised connectors, not browse the server
owner's library.

For one personal user a private editor can use the existing single-user engine.
For many customers, plan explicit project/session ownership and isolated workers.
The VPS could host the frontend, accounts, database and queues while compute
workers run locally or remotely according to measured demand.

Private use does not itself create advertising income. AdSense would require
approved public pages and a real audience. Original tutorials and template
discovery pages are a more suitable initial advertising surface than private
research or an editor full of download/generate controls. Keep any future GPU
budget separate from hoped-for ad earnings until revenue actually arrives.

## Future optional RunPod funding

A later design can connect a funded generation allowance to a RunPod Serverless
queue endpoint. The VPS checks entitlement, reserves the maximum permitted job
cost, submits once and records the remote job ID. Completion processing retrieves
the result and settles the reservation exactly once; polling/retries must not
submit a second billable job. Credentials remain on the server.

RunPod Flex workers can scale to zero; endpoint settings allow active workers
to be zero and a maximum worker count to bound concurrency. Initial trial
settings should permit one worker, a short idle timeout and a measured execution
timeout. These are proposed settings, not resources created here.
[Endpoint settings](https://docs.runpod.io/serverless/endpoints/endpoint-configurations).

Compute billing includes worker startup, execution and the idle period before
shutdown. Storage is separate. Benchmark the exact model and include failed-job
costs before setting customer credit prices. A fixed allowance or prepaid
top-ups offers a clearer cost boundary than unlimited generation.
[RunPod billing](https://docs.runpod.io/serverless/pricing).

Only add this after funding is authorised and the model/provider deployment
rights are reviewed. The current publishing profile does not collect RunPod
keys, sell generation credits or launch GPU work.
