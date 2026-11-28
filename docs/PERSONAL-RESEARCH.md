# Personal Research and Reference Desk

Version 1.21 connects the desktop and local browser editor to saved Reference
Desk output. Choose **Bring in research** from Home or **Research** in the
navigation. No account, model key or GPU is needed for these tools.

## Working with saved files

1. Use **Open Reference Desk** to open the installed personal app. Create the
   transcript, SRT, audio or video there using sources you are authorised to use.
2. Back in Canvas Studio, choose **Refresh saved files**. The library reads the
   latest 250 supported files in Reference Desk's Transcripts, Audio and Video
   subfolders. It follows Reference Desk's configured output directory when that
   directory is valid; otherwise it uses `~/Reference Desk`.
3. Preview a transcript, then choose **Copy source into project**. Select a
   passage and add it to the current page's notes. The citation retains the
   source title, original link where available and passage timestamp.
4. Import an SRT source and choose **Add captions to video**. Captions use the
   shared timeline and remain editable. Review timing against the recording.
5. Import audio/video into the video library, then add it to the edit at the
   playhead. Original files stay where Reference Desk saved them.

**Import transcript / SRT** and **Import audio / video** also accept files from
other folders. These are the portable route when Reference Desk is absent, or
when a browser client and the editor engine run on different computers.

## Copies and privacy

Import is explicit. The library does not watch folders, start transcription or
download remote media. Reading the library does not change a project. Preview
text is not a saved project source until copied. The browser rejects a transcript
that changed on disk between preview and import.

The project stores a source snapshot, filename, source URL/method when present,
timestamps and SHA-256 identity of the imported bytes. Audio/video originals are
copied into the existing private content-addressed media store. The file library
accepts only supported files in the scoped subfolders; traversal and symbolic
links are rejected.

Imported sources travel inside the editable `.canvas` file. Share that file only
when its research and media are intended for the recipient. PDF/PNG delivery
does not automatically publish a separate research library. **Remove project
copy** removes the research card and is undoable; it does not delete Reference
Desk files, existing page notes or timeline clips.

## Current limits

- UTF-8 TXT/SRT below 1 MB each; at most 2,000 timestamped passages per source.
- At most 100 sources and 4 MB of raw transcripts / 8 MB including passage text
  in one project. Page notes support 100,000 bytes.
- Imported media below 128 MiB per file and the existing 256 MiB portable film
  total. Large recordings may need a shorter or compressed working copy.
- Video captions use the existing limit of 200 clips and a ten-minute timeline.
  Longer transcripts can remain research sources; use a shorter SRT for a film.
- TXT timestamps retain source passage starts. They do not invent caption end
  times. Use SRT for timed caption import.
- Reference Desk's speech recognition and caption grouping can be imperfect.
  Imported transcription is a source for review, not verified factual research.

## Portable compatibility and validation

Research projects use portable envelope version 6 and require
`research-sources-v1`. Older readers reject them instead of silently dropping
the source collection. Projects without research retain their existing format;
old projects still load with an empty source collection.

Engine checks cover parsing, size/timing validation, scoped paths and links,
changed-preview rejection, cited notes, duplicate imports, atomic caption
overlap failure and portable-version guards. Browser acceptance imports actual
generated audio/video, renders a caption frame, tests undo/redo, opens the saved
file in a fresh native process, checks document isolation and verifies the input
files are unchanged. Native window proofs cover Research and Home at two sizes.

`CANVAS_STUDIO_REFERENCE_ROOT` is an explicit local override for testing or a
chosen personal library. It is not a request parameter and does not turn this
local file bridge into a public file-browsing service.
