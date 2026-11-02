# Canvas Studio: practical acceptance scenarios

Purpose: establish usefulness for personal print and digital product work. These are acceptance requirements, not claims that the present application passes them.

## 1. Included templates actually export

- Open every built-in production template and make a visible edit.
- Save, close and reopen it with its assets.
- Export PDF and PNG at the intended size.
- Open/render both outputs independently and compare them to the editor.
- Check unsupported features before export, with an actionable layer reference.

Pass: no default production template has fatal export errors, missing elements or unexplained visual changes.

## 2. A reusable motif behaves as one object

- Group a shape, a text label and an image.
- Copy/paste and duplicate the complete group.
- Move, resize and rotate the copy; lock/unlock and undo/redo it.
- Save/reopen and repeat an edit.

Pass: all parts stay together; copied groups have fresh identities; the original is unchanged. Clipboard behaviour is explicit for multiple selections.

## 3. A stationery collection takes one brief

- Create coordinated invitation, programme, welcome sign, table card and newspaper designs.
- Use an imported local font and mixed emphasis within one paragraph.
- Supply names, date and location once; include a deliberately long value.
- Change one shared accent colour and heading style.
- Save the set as a master and create a fresh working copy.
- Export the set and inspect all pages.

Pass: linked fields/styles update intentionally; long content is flagged or fitted within an agreed readable limit; the master remains unchanged.

## 4. A book survives normal revisions

- Assemble a representative 50-page illustrated workbook from local artwork.
- Apply a master footer, automatic page numbers and facing-page margin rules.
- Reorder a section, insert a blank page and replace an image.
- Save, close, reopen and export the full book and a page range.
- Measure open/save/export time, interaction responsiveness and peak memory on the target computer.

Pass: no content loss, correct page order/numbering/margins, reproducible dimensions and an independently inspected PDF. Agree performance targets from the representative document rather than an empty test file.

## 5. Batch personalization catches mistakes

- Import a 50-row CSV with normal names, accented characters, a long name and a missing required value.
- Bind its columns to template fields and preview records.
- Export named files and a completion/error summary.
- Correct one record and regenerate it without changing the master.

Pass: required-value failures and overflow are identified; outputs are complete, clearly named and traceable to records. No silent clipping or overwriting.

## 6. A puzzle and its answer key agree

- Generate a small seeded word-search collection using declared difficulty rules.
- Verify required words/placements and answer keys automatically and by inspection.
- Change the visual theme, save/reopen and regenerate from the saved specification.

Pass: the same seed/settings reproduce the puzzle; solutions match; styling edits do not change puzzle data accidentally.

## 7. A recipient can use an advertised editable file

- Choose the actual promised route: native file, an interchange format, a fillable PDF or browser editing.
- Give a fresh recipient/process only the delivered package or link.
- Change text and a photo, save/reopen and export.
- Check fonts, dimensions, assets and layout without relying on the creator's machine.

Pass: the recipient completes the promised changes without developer intervention. A finished PDF or HTML slideshow must not be treated as evidence of arbitrary layout editing.

## Evidence to keep for each accepted workflow

Keep the input fixture, saved editable result, output files, independent render screenshots, application version, relevant settings and a short pass/fail note. Record limitations explicitly. Only add automated tests that protect a meaningful failure or required behaviour.
